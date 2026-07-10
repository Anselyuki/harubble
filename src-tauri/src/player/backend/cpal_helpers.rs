//! CPAL 音频输出格式映射与纯计算辅助函数。
//!
//! 本模块收录对 CPAL `SampleFormat`、`OutputSampleFormat` 以及流配置范围做纯计算的辅助函数，
//! 供 CPAL 后端在格式协商与输出流构建时复用，同时使单元测试无需依赖设备即可独立覆盖这些逻辑。

use crate::player::backend::OutputSampleFormat;
use cpal::{SampleFormat, SupportedStreamConfig, SupportedStreamConfigRange};

/// 输出平滑渐变所参考的帧数。
///
/// 每次增益步进以该值为除数，使音频输出在静音与播放之间做线性渐变，
/// 避免硬切边界产生的爆音。
pub(crate) const OUTPUT_SMOOTHING_FRAMES: usize = 64;

/// 返回 CPAL 采样格式的优先级序号。
///
/// 序号越小越优先；不受支持的格式返回 `None`，用于在候选列表中过滤掉不可用格式。
pub(crate) fn sample_format_priority(format: SampleFormat) -> Option<usize> {
    match format {
        SampleFormat::F32 => Some(0),
        SampleFormat::F64 => Some(1),
        SampleFormat::I16 => Some(2),
        SampleFormat::U16 => Some(3),
        SampleFormat::I24 => Some(4),
        SampleFormat::U24 => Some(5),
        SampleFormat::I32 => Some(6),
        SampleFormat::U32 => Some(7),
        SampleFormat::I8 => Some(8),
        SampleFormat::U8 => Some(9),
        SampleFormat::I64 => Some(10),
        SampleFormat::U64 => Some(11),
        _ => None,
    }
}

/// 将 CPAL 采样格式转换为播放后端的输出采样格式枚举。
///
/// 返回 `None` 表示该格式当前不受后端支持。
pub(crate) fn output_sample_format(format: SampleFormat) -> Option<OutputSampleFormat> {
    match format {
        SampleFormat::F32 => Some(OutputSampleFormat::F32),
        SampleFormat::F64 => Some(OutputSampleFormat::F64),
        SampleFormat::I8 => Some(OutputSampleFormat::I8),
        SampleFormat::I16 => Some(OutputSampleFormat::I16),
        SampleFormat::I24 => Some(OutputSampleFormat::I24),
        SampleFormat::I32 => Some(OutputSampleFormat::I32),
        SampleFormat::I64 => Some(OutputSampleFormat::I64),
        SampleFormat::U8 => Some(OutputSampleFormat::U8),
        SampleFormat::U16 => Some(OutputSampleFormat::U16),
        SampleFormat::U24 => Some(OutputSampleFormat::U24),
        SampleFormat::U32 => Some(OutputSampleFormat::U32),
        SampleFormat::U64 => Some(OutputSampleFormat::U64),
        _ => None,
    }
}

/// 返回指定 CPAL 采样格式对应的 TPDF 抖动幅度（以 1 LSB 归一化值表示）。
///
/// 浮点格式不需要抖动，返回 `0.0`；整数格式按位深折算出 1 LSB 对应的归一化幅度。
pub(crate) fn output_dither_lsb(format: SampleFormat) -> f32 {
    match format {
        SampleFormat::F32 | SampleFormat::F64 => 0.0,
        SampleFormat::I8 | SampleFormat::U8 => 1.0 / 128.0,
        SampleFormat::I16 | SampleFormat::U16 => 1.0 / 32_768.0,
        SampleFormat::I24 | SampleFormat::U24 => 1.0 / 8_388_608.0,
        SampleFormat::I32 | SampleFormat::U32 => 1.0 / 2_147_483_648.0,
        SampleFormat::I64 | SampleFormat::U64 => 1.0 / 9_223_372_036_854_775_808.0,
        _ => 0.0,
    }
}

/// 将播放后端输出采样格式枚举转换回对应的 CPAL `SampleFormat`。
pub(crate) fn cpal_sample_format(format: OutputSampleFormat) -> SampleFormat {
    match format {
        OutputSampleFormat::F32 => SampleFormat::F32,
        OutputSampleFormat::F64 => SampleFormat::F64,
        OutputSampleFormat::I8 => SampleFormat::I8,
        OutputSampleFormat::I16 => SampleFormat::I16,
        OutputSampleFormat::I24 => SampleFormat::I24,
        OutputSampleFormat::I32 => SampleFormat::I32,
        OutputSampleFormat::I64 => SampleFormat::I64,
        OutputSampleFormat::U8 => SampleFormat::U8,
        OutputSampleFormat::U16 => SampleFormat::U16,
        OutputSampleFormat::U24 => SampleFormat::U24,
        OutputSampleFormat::U32 => SampleFormat::U32,
        OutputSampleFormat::U64 => SampleFormat::U64,
    }
}

/// 判断指定 CPAL 采样格式是否受当前后端支持。
///
/// 仅当 `sample_format_priority` 返回 `Some` 时视为受支持。
pub(crate) fn is_supported_sample_format(format: SampleFormat) -> bool {
    sample_format_priority(format).is_some()
}

/// 判断指定 CPAL 流配置是否满足后端的最低可用要求。
///
/// 须同时满足：采样格式受支持、通道数大于零、采样率大于零。
pub(crate) fn is_supported_output_config(config: &SupportedStreamConfig) -> bool {
    is_supported_sample_format(config.sample_format())
        && config.channels() > 0
        && config.sample_rate() > 0
}

/// 将源采样率限制在指定配置范围的 `[min, max]` 区间内。
pub(crate) fn clamp_sample_rate(config: &SupportedStreamConfigRange, source_rate: u32) -> u32 {
    source_rate
        .max(config.min_sample_rate())
        .min(config.max_sample_rate())
}

/// 计算源采样率与指定配置范围内最近可用采样率之间的绝对差值。
///
/// 用于在候选配置中选取采样率最接近源格式的条目。
pub(crate) fn sample_rate_distance(config: &SupportedStreamConfigRange, source_rate: u32) -> u32 {
    clamp_sample_rate(config, source_rate).abs_diff(source_rate)
}

/// 将浮点增益值向目标值步进一帧。
///
/// 每次调用增益变化量为 `1 / OUTPUT_SMOOTHING_FRAMES`，
/// 确保输出音量在渐变期间不会过冲目标值。
pub(crate) fn step_toward(current: f32, target: f32) -> f32 {
    let step = 1.0 / OUTPUT_SMOOTHING_FRAMES as f32;
    if current < target {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    }
}

/// 将浮点 PCM 样本裁剪到 `[-1.0, 1.0]`，非有限值归零。
///
/// 在写入输出缓冲前调用，防止非法值（NaN、Inf）传入驱动造成异常。
pub(crate) fn sanitize_output_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}
