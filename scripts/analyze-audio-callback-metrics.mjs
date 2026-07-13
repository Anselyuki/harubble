#!/usr/bin/env node
/**
 * P1-6 音频回调 metrics 分析器
 *
 * 用法：
 *   bun run scripts/analyze-audio-callback-metrics.mjs [logfile]
 *
 * 缺省从 macOS 默认位置加载：
 *   ~/Library/Application Support/com.harubble.app/logs/persistent.jsonl
 *
 * 采集流程：
 *   1. 启动应用（可用 debug 或 release build）
 *   2. 播放一首完整曲目（建议 30 分钟连续播放）
 *   3. 正常退出（触发 flush_logs_on_exit）
 *   4. 运行本脚本
 *
 * 输出：
 *   - 总回调次数 / 平均耗时 / 最大耗时
 *   - log2μs 直方图分布
 *   - 近似 P50/P95/P99（按桶累计）
 *   - underrun / silence_due_to_lock 累计
 *   - 对照 audit doc §6 决策矩阵给出建议
 */

import { readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const DEFAULT_LOG_PATH = join(
  homedir(),
  'Library',
  'Application Support',
  'com.harubble.app',
  'logs',
  'persistent.jsonl'
);

const args = process.argv.slice(2);
const logPath = args[0] ?? DEFAULT_LOG_PATH;

let raw;
try {
  raw = readFileSync(logPath, 'utf8');
} catch (err) {
  console.error(`无法读取日志：${logPath}`);
  console.error(err.message);
  process.exit(1);
}

// 解析所有 audio.callback_metrics 条目
const entries = raw
  .split('\n')
  .filter(Boolean)
  .map((line) => {
    try {
      return JSON.parse(line);
    } catch {
      return null;
    }
  })
  .filter((e) => e && e.code === 'audio.callback_metrics');

if (entries.length === 0) {
  console.log('未找到 audio.callback_metrics 条目。');
  console.log('确认：');
  console.log('  1. 应用已启动并播放过音乐');
  console.log('  2. 应用正常退出（触发 flush_logs_on_exit）');
  console.log('  3. 日志级别 <= Debug（默认已满足）');
  process.exit(0);
}

// 聚合所有条目
let totalCallbacks = 0;
let totalElapsedNs = 0;
let totalUnderrunFrames = 0;
let totalSilence = 0;
let peakElapsedNs = 0;
const bucketSums = new Array(16).fill(0);

const legacyEntries = []; // 缺少 callback_count 字段（P1-6 前的旧数据）
const modernEntries = [];

for (const entry of entries) {
  const ctx = entry.context ?? {};
  const callbackCount = ctx['audio.callback_count'];
  if (typeof callbackCount === 'number') {
    modernEntries.push(entry);
    totalCallbacks += callbackCount;
    totalElapsedNs += ctx['audio.callback_elapsed_ns_total'] ?? 0;
    peakElapsedNs = Math.max(
      peakElapsedNs,
      ctx['audio.callback_elapsed_ns_max'] ?? 0
    );
    const buckets = ctx['audio.callback_duration_buckets'] ?? [];
    for (let i = 0; i < Math.min(buckets.length, bucketSums.length); i++) {
      bucketSums[i] += buckets[i];
    }
  } else {
    legacyEntries.push(entry);
  }
  totalUnderrunFrames += ctx['audio.callback_underrun_frames'] ?? 0;
  totalSilence += ctx['audio.callback_silence_due_to_lock'] ?? 0;
}

console.log(`日志：${logPath}`);
console.log(`总 audio.callback_metrics 条目：${entries.length}`);
if (legacyEntries.length > 0) {
  console.log(
    `  其中 ${legacyEntries.length} 条为 P1-6 前的旧数据（仅 silence/underrun）`
  );
}
console.log(`  其中 ${modernEntries.length} 条为完整 P1-6 数据`);
console.log();

if (modernEntries.length === 0) {
  console.log('没有 P1-6 完整数据；跳过后续统计。');
  console.log('用新构建的应用播放一次音乐后再运行本脚本即可。');
  process.exit(0);
}

console.log('=== 汇总统计 ===');
console.log(`总回调次数：${totalCallbacks.toLocaleString()}`);
const avgNs = totalCallbacks > 0 ? totalElapsedNs / totalCallbacks : 0;
console.log(`平均运行时间：${(avgNs / 1000).toFixed(2)} μs`);
console.log(`历史峰值：${(peakElapsedNs / 1000).toFixed(2)} μs`);
console.log(`Silence due to lock 累计：${totalSilence}`);
console.log(`Underrun 累计帧数：${totalUnderrunFrames}`);
console.log();

// 计算百分位（log2μs 桶累计法）
console.log('=== 运行时间分布（log2μs 桶）===');
console.log(
  `${'桶'.padEnd(4)} ${'区间'.padEnd(18)} ${'计数'.padStart(12)} ${'累计占比'.padStart(10)}`
);
let cumulative = 0;
const totalBucketCount = bucketSums.reduce((a, b) => a + b, 0);
const percentileTargets = [50, 95, 99];
const percentileHits = {};
for (let i = 0; i < bucketSums.length; i++) {
  const lo = 1 << i;
  const hi = 1 << (i + 1);
  const label = i === 15 ? `≥${lo} μs` : `[${lo}, ${hi}) μs`;
  cumulative += bucketSums[i];
  const pct = totalBucketCount > 0 ? (cumulative / totalBucketCount) * 100 : 0;
  console.log(
    `${String(i).padEnd(4)} ${label.padEnd(18)} ${bucketSums[i]
      .toLocaleString()
      .padStart(12)} ${pct.toFixed(1).padStart(9)}%`
  );
  for (const target of percentileTargets) {
    if (!(target in percentileHits) && pct >= target) {
      percentileHits[target] = hi;
    }
  }
}
console.log();

console.log('=== 近似百分位（桶上界，log2μs 精度）===');
for (const target of percentileTargets) {
  const val = percentileHits[target];
  console.log(
    `  P${target}：${val !== undefined ? `< ${val} μs` : '未达到（数据不足）'}`
  );
}
console.log();

// 对照 audit doc §6 决策矩阵
console.log('=== 决策矩阵（对照 audit doc §6）===');
// 需要 buffer_period（假设 48kHz 512 帧 = 10.6ms 或 128 帧 = 2.7ms）
const bufferPeriodMs = 10.6; // 默认按 48k/512 估算
console.log(`  假设 buffer period ≈ ${bufferPeriodMs} ms（48kHz/512）`);
const p99Us = percentileHits[99] ?? 0;
const p99Ratio = p99Us / 1000 / bufferPeriodMs;
console.log(`  P99 / buffer_period ≈ ${p99Ratio.toFixed(2)}`);

// hourly 归一化（假设 100Hz 采样率上报，每秒回调数量与 buffer_period 有关）
// 简化：直接按累计条目估算，无法拿到完整会话时长
const underrunPerCallback =
  totalCallbacks > 0 ? totalUnderrunFrames / totalCallbacks : 0;
console.log(`  Underrun / callback：${underrunPerCallback.toFixed(6)}`);
console.log();

if (totalUnderrunFrames === 0 && totalSilence === 0 && p99Ratio < 0.4) {
  console.log('✅ 结论：当前实现足够，不改造（audit §6 第一档）');
} else if (totalUnderrunFrames <= 10 && p99Ratio < 0.7) {
  console.log('📊 结论：记录基线，暂不改造（audit §6 第二档）');
} else if (p99Ratio < 0.7) {
  console.log(
    '⚠️  结论：软改造（audit §6 第三档）— scratch 预分配 + current_error 换 arc-swap'
  );
} else {
  console.log(
    '🔥 结论：硬改造（audit §6 第四档）— SPSC ring buffer + park_timeout'
  );
}
console.log();
console.log(
  '（注：真实评估需要 30 分钟连续播放数据，且按目标设备/采样格式分别测量）'
);
