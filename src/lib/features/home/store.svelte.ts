import type {
  SeriesGroup,
  HistoryEntry,
  HomepageStatus,
  TagDimension,
  TagGroup,
} from '$lib/types';

let seriesGroups = $state<SeriesGroup[]>([]);
let recentHistory = $state<HistoryEntry[]>([]);
let status = $state<HomepageStatus | null>(null);
let loading = $state(false);
let belongReady = $state(false);
let tagDimensions = $state<TagDimension[]>([]);
let tagGroups = $state<TagGroup[]>([]);
let selectedDimensionKey = $state<string | null>(null);

function reset() {
  seriesGroups = [];
  recentHistory = [];
  status = null;
  loading = false;
  belongReady = false;
  tagDimensions = [];
  tagGroups = [];
  selectedDimensionKey = null;
}

export const homeStore = {
  get seriesGroups() {
    return seriesGroups;
  },
  set seriesGroups(value: SeriesGroup[]) {
    seriesGroups = value;
  },
  get recentHistory() {
    return recentHistory;
  },
  set recentHistory(value: HistoryEntry[]) {
    recentHistory = value;
  },
  get status() {
    return status;
  },
  set status(value: HomepageStatus | null) {
    status = value;
  },
  get loading() {
    return loading;
  },
  set loading(value: boolean) {
    loading = value;
  },
  get belongReady() {
    return belongReady;
  },
  set belongReady(value: boolean) {
    belongReady = value;
  },
  get tagDimensions() {
    return tagDimensions;
  },
  set tagDimensions(value: TagDimension[]) {
    tagDimensions = value;
  },
  get tagGroups() {
    return tagGroups;
  },
  set tagGroups(value: TagGroup[]) {
    tagGroups = value;
  },
  get selectedDimensionKey() {
    return selectedDimensionKey;
  },
  set selectedDimensionKey(value: string | null) {
    selectedDimensionKey = value;
  },
  reset,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    reset();
  });
}
