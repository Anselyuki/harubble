#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import * as OpenCC from 'opencc-js';

const ARTIST_ID = 1519954412; // 塞壬唱片-MSR
const API_BASE = 'https://itunes.apple.com';
const registryPath = resolve(import.meta.dirname, '../data/tag_registry.json');

const converter = OpenCC.Converter({ from: 't', to: 'cn' });

function extractCore(name) {
  return name
    .replace(/\s*-\s*(Single|EP)$/i, '')
    .replace(/\s*\(Original Soundtrack\s*\d*\)/i, '')
    .replace(/\s*\(Deluxe Edition\)/i, '')
    .replace(/【[^】]*】/g, '')
    .replace(/《[^》]*》/g, '')
    .replace(/\s*OST\d*\s*$/i, '')
    .replace(/[:：]\s*.+$/, '')
    .trim();
}

function normalizeForMatch(name) {
  return name
    .replace(/\s+/g, '')
    .replace(/[·・-]/g, '')
    .toLowerCase();
}

async function searchAlbumDate(name) {
  const query = encodeURIComponent(`塞壬唱片 ${name}`);
  const url = `${API_BASE}/search?term=${query}&entity=album&country=cn&limit=10`;
  const res = await fetch(url);
  if (!res.ok) return null;
  const data = await res.json();
  const albums = data.results ?? [];
  const core = normalizeForMatch(extractCore(name));
  for (const album of albums) {
    const itunesCore = normalizeForMatch(
      converter(extractCore(album.collectionName))
    );
    if (itunesCore.includes(core) || core.includes(itunesCore)) {
      return album.releaseDate.split('T')[0];
    }
  }
  return null;
}

console.log('Fetching albums from iTunes (artist: 塞壬唱片-MSR)...');
const res = await fetch(
  `${API_BASE}/lookup?id=${ARTIST_ID}&entity=album&country=cn&limit=200`
);
if (!res.ok) {
  console.error(`iTunes API failed: ${res.status}`);
  process.exit(1);
}

const body = await res.json();
const itunesAlbums = body.results.filter((r) => r.wrapperType === 'collection');
console.log(`Found ${itunesAlbums.length} albums on Apple Music.`);

const itunesMap = new Map();
for (const album of itunesAlbums) {
  const core = extractCore(album.collectionName);
  const simplified = converter(core);
  const key = normalizeForMatch(simplified);
  if (!itunesMap.has(key)) {
    itunesMap.set(key, {
      date: album.releaseDate.split('T')[0],
      originalName: album.collectionName,
    });
  }
}

const raw = readFileSync(registryPath, 'utf8');
const registry = JSON.parse(raw);

let updated = 0;
const needSearch = [];

for (const entry of registry.albums) {
  if (entry.releaseDate) continue;
  if (!entry.name) continue;

  const core = extractCore(entry.name);
  const key = normalizeForMatch(core);

  const match = itunesMap.get(key);
  if (match) {
    entry.releaseDate = match.date;
    updated++;
  } else {
    needSearch.push(entry);
  }
}

if (needSearch.length > 0) {
  console.log(
    `\nSearching iTunes for ${needSearch.length} unmatched album(s)...`
  );
  const unmatched = [];
  for (const entry of needSearch) {
    const date = await searchAlbumDate(extractCore(entry.name));
    if (date) {
      entry.releaseDate = date;
      updated++;
    } else {
      unmatched.push(entry.name);
    }
    await new Promise((r) => setTimeout(r, 300));
  }
  if (unmatched.length > 0) {
    console.log(`\nStill unmatched (${unmatched.length}):`);
    for (const name of unmatched) {
      console.log(`  - ${name}`);
    }
  }
}

if (updated > 0) {
  registry.albums.sort((a, b) => {
    const dateA = a.releaseDate;
    const dateB = b.releaseDate;
    if (dateA && !dateB) return -1;
    if (!dateA && dateB) return 1;
    if (dateA && dateB && dateA !== dateB) return dateA.localeCompare(dateB);
    const numA = parseInt(a.cid, 10);
    const numB = parseInt(b.cid, 10);
    if (numA !== numB) return numA - numB;
    return a.cid.localeCompare(b.cid);
  });
  writeFileSync(registryPath, JSON.stringify(registry, null, 2) + '\n', 'utf8');
}

console.log(`\nUpdated ${updated} release date(s).`);
