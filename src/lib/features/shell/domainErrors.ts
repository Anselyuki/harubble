import * as m from '$lib/paraglide/messages.js';
import type {
  CollectionError,
  DownloadError,
  LibraryError,
  SearchError,
  TagEditorError,
} from '$lib/types';

/**
 * 判断值是否为 IPC 结构化域错误负载。
 *
 * 后端 P0-4 引入的 CollectionError / LibraryError / SearchError / DownloadError /
 * TagEditorError 都以 `{ code, detail? }` 形状序列化到前端；此函数仅检查最小
 * 特征（存在 `code` 字符串），不区分具体域，供 formatXxxError 分域细化。
 */
function isDomainErrorPayload(
  value: unknown
): value is { code: string; detail?: unknown } {
  return (
    typeof value === 'object' &&
    value !== null &&
    'code' in value &&
    typeof (value as { code: unknown }).code === 'string'
  );
}

/**
 * 把 Collection 域 IPC 错误转换为本地化用户提示。
 *
 * 匹配 P0-4 定义的 CollectionError code；未识别的 code 或非结构化错误一律走
 * 通用兜底文案，避免把 Rust 内部细节（如 `"lock poisoned"`、SQLite 报错串）
 * 直接暴露给用户。
 */
export function formatCollectionError(error: unknown): string {
  if (isDomainErrorPayload(error)) {
    const code = error.code as CollectionError['code'];
    switch (code) {
      case 'notFound':
        return m.domain_collection_error_not_found();
      case 'readOnly':
        return m.domain_collection_error_read_only();
      case 'database':
        return m.domain_collection_error_database();
      case 'serialization':
        return m.domain_collection_error_serialization();
      case 'unsupportedVersion':
        return m.domain_collection_error_unsupported_version();
    }
  }
  return m.domain_generic_error();
}

/**
 * 把 Library 域 IPC 错误转换为本地化用户提示。
 */
export function formatLibraryError(error: unknown): string {
  if (isDomainErrorPayload(error)) {
    const code = error.code as LibraryError['code'];
    switch (code) {
      case 'network':
        return m.domain_library_error_network();
      case 'notFound':
        return m.domain_library_error_not_found();
      case 'internal':
        return m.domain_generic_error();
    }
  }
  return m.domain_generic_error();
}

/**
 * 把 Search 域 IPC 错误转换为本地化用户提示。
 */
export function formatSearchError(error: unknown): string {
  if (isDomainErrorPayload(error)) {
    const code = error.code as SearchError['code'];
    switch (code) {
      case 'notReady':
        return m.domain_search_error_not_ready();
      case 'internal':
        return m.domain_generic_error();
    }
  }
  return m.domain_generic_error();
}

/**
 * 把 Download 域 IPC 错误转换为本地化用户提示。
 */
export function formatDownloadError(error: unknown): string {
  if (isDomainErrorPayload(error)) {
    const code = error.code as DownloadError['code'];
    switch (code) {
      case 'notFound':
        return m.domain_download_error_not_found();
      case 'network':
        return m.domain_download_error_network();
      case 'io':
        return m.domain_download_error_io();
      case 'invalidState':
        return m.domain_download_error_invalid_state();
      case 'internal':
        return m.domain_generic_error();
    }
  }
  return m.domain_generic_error();
}

/**
 * 把 TagEditor 域 IPC 错误转换为本地化用户提示。
 */
export function formatTagEditorError(error: unknown): string {
  if (isDomainErrorPayload(error)) {
    const code = error.code as TagEditorError['code'];
    switch (code) {
      case 'io':
        return m.domain_tag_editor_error_io();
      case 'serialization':
        return m.domain_tag_editor_error_serialization();
      case 'unsupportedVersion':
        return m.domain_tag_editor_error_unsupported_version();
      case 'internal':
        return m.domain_generic_error();
    }
  }
  return m.domain_generic_error();
}
