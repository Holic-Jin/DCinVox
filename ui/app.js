// DCinVox UI — 빌드 도구 없이 동작하는 순수 JS (번들 최소화)
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const dialog = window.__TAURI__.dialog;

const VIDEO_EXTS = ["mp4", "mov", "mkv", "webm", "avi", "m4v", "flv", "wmv", "ts"];
const $ = (sel) => document.querySelector(sel);

const state = {
  project: null,
  mediaBase: null, // 로컬 재생 서버 주소

  selectedId: null,
  busy: false,
  view: "transcript", // 아래 패널: song(음계) | result(생성 결과) | transcript(인식 결과)
  playerSrc: null, // 플레이어에 올라간 것: asset id, "preview"(생성), "converted"(변환)
  // 단계 상태: locked | ready | working | done | error
  steps: { analyze: "locked", generate: "locked", convert: "locked", export: "locked" },
};

// ---------- 공통 ----------
let toastTimer;
function toast(msg, isError = false) {
  const el = $("#toast");
  el.textContent = msg;
  el.classList.toggle("error", isError);
  el.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (el.hidden = true), isError ? 6000 : 3000);
}

function setStatus(text) {
  $("#status").textContent = text || "";
}

function fmtTime(sec) {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
}

// ---------- 프로젝트 ----------
async function chooseProject() {
  const dir = await dialog.open({ directory: true, title: "프로젝트 폴더 선택" });
  if (!dir) return;
  try {
    setProject(await invoke("open_project", { path: dir }));
    $("#welcome").hidden = true;
    $("#app").hidden = false;
  } catch (e) {
    toast(String(e), true);
  }
}

function setProject(project) {
  state.project = project;
  $("#project-name").textContent = project.name;
  $("#lyrics").value = project.lyrics;
  if (!project.assets.some((a) => a.id === state.selectedId)) state.selectedId = null;
  // 모든 영상이 분석돼 있으면 분석 단계 완료 상태로 복원
  // 음성 인식 + 발음 정렬 + 조각 미리 듣기까지 끝나야 분석 완료
  const allAnalyzed = project.assets.length > 0 && project.assets.every(isFullyAnalyzed);
  if (allAnalyzed && state.steps.analyze !== "done") state.steps.analyze = "done";
  if (!allAnalyzed && state.steps.analyze === "done") resetStepsFrom("analyze");
  // 지금 가사로 만든 생성 결과가 있으면 생성 완료 상태로 복원
  const gen = project.generation;
  if (allAnalyzed && gen && gen.lyrics === project.lyrics && state.steps.generate === "locked") {
    state.steps.generate = "done";
  }
  // 지금 음계로 변환한 결과가 있으면 변환 완료 상태로 복원
  if (state.steps.generate === "done" && project.conversion && state.steps.convert === "locked") {
    state.steps.convert = "done";
  }
  renderAssets();
  refreshSteps();
  renderNotes();
}

const isFullyAnalyzed = (a) => a.analysis === "done" && a.aligned && a.checked;

// ---------- 영상 목록 ----------
function renderAssets() {
  const list = $("#asset-list");
  list.replaceChildren();
  const assets = state.project?.assets ?? [];
  $("#asset-empty").hidden = assets.length > 0;

  for (const a of assets) {
    const li = document.createElement("li");
    li.classList.toggle("selected", a.id === state.selectedId);
    li.title = a.file_name;

    const name = document.createElement("span");
    name.className = "name";
    name.textContent = a.file_name;
    const dur = document.createElement("span");
    dur.className = "dur";
    dur.textContent = fmtTime(a.duration);
    const rm = document.createElement("button");
    rm.className = "remove";
    rm.textContent = "✕";
    rm.title = "목록에서 빼기 (파일은 Assets 폴더에 남습니다)";
    rm.addEventListener("click", (ev) => {
      ev.stopPropagation();
      removeAsset(a.id);
    });

    const badge = document.createElement("span");
    const needsAlign = a.analysis === "done" && !isFullyAnalyzed(a);
    badge.className = `badge ${needsAlign ? "" : a.analysis}`;
    badge.textContent = needsAlign ? "½" : a.analysis === "done" ? "✓" : a.analysis === "error" ? "!" : "";
    badge.title = needsAlign
      ? "일부만 분석됨 — 분석을 누르면 남은 단계(발음 정렬·조각 미리 듣기)를 이어서 합니다"
      : a.analysis === "done" ? "분석 완료" : a.analysis === "error" ? "분석 실패 — 분석을 다시 누르면 재시도" : "분석 전";

    li.append(badge, name, dur, rm);
    li.addEventListener("click", () => selectAsset(a.id));
    list.append(li);
  }
}

// 프로젝트 기준 상대경로 → 로컬 재생 서버 URL
function mediaUrl(relPath) {
  return `${state.mediaBase}/${relPath.split("/").map(encodeURIComponent).join("/")}`;
}

async function selectAsset(id) {
  state.selectedId = id;
  state.view = "transcript";
  const asset = state.project.assets.find((a) => a.id === id);
  await loadPlayer(`Assets/${asset.file_name}`, id);
  renderAssets();
  renderNotes();
}

// 플레이어에 파일을 올리고 재생 준비될 때까지 대기
async function loadPlayer(relPath, key, bust = false) {
  state.mediaBase ??= await invoke("media_base");
  const player = $("#player");
  clipStopAt = null;
  state.playerSrc = key;
  player.src = mediaUrl(relPath) + (bust ? `?v=${Date.now()}` : "");
  $("#player-empty").hidden = true;
  // 불러오기 실패 이벤트가 오지 않는 경우도 있어 3초 뒤에는 그냥 진행 (화면이 멈추지 않게)
  await new Promise((resolve) => {
    if (player.readyState >= 1) return resolve();
    player.addEventListener("loadedmetadata", resolve, { once: true });
    player.addEventListener("error", resolve, { once: true });
    setTimeout(resolve, 3000);
  });
}

const loadPreview = () => loadPlayer("cache/preview.mp4", "preview", true);
const loadConverted = () => loadPlayer("cache/converted.mp4", "converted", true);

// ---------- 아래 패널: 생성 결과 / 인식 결과 ----------
for (const tab of document.querySelectorAll(".tabs button")) {
  tab.addEventListener("click", async () => {
    state.view = tab.dataset.tab;
    if (state.view === "result" && state.project?.generation && state.playerSrc !== "preview") await loadPreview();
    if (state.view === "song" && state.project?.conversion && state.playerSrc !== "converted") await loadConverted();
    else if (state.view === "song" && state.project?.generation && !state.project?.conversion && state.playerSrc !== "preview") await loadPreview();
    renderNotes();
    if (state.view === "song") scrollRollToNotes();
  });
}

function showNotesEmpty(text) {
  $("#notes-empty").textContent = text;
  $("#notes-empty").hidden = false;
}

function renderNotes() {
  for (const tab of document.querySelectorAll(".tabs button")) {
    tab.classList.toggle("active", tab.dataset.tab === state.view);
  }
  $("#clips").hidden = true;
  $("#transcript").hidden = true;
  $("#song").hidden = true;
  $("#notes-empty").hidden = true;
  if (state.view === "song") {
    $("#notes-title").textContent = "음계 — 블록을 위아래로 옮기면 음이 바뀝니다";
    if (state.project?.song) renderSongEditor();
    else showNotesEmpty("생성을 마치면 가사 음절마다 블록이 생깁니다.");
  } else if (state.view === "result") {
    const gen = state.project?.generation;
    $("#notes-title").textContent = "생성 결과";
    if (gen) renderClips(gen);
    else showNotesEmpty("가사를 쓰고 생성을 누르면 여기에 가사 조각이 나타납니다.");
  } else {
    renderTranscript();
  }
}

const KIND_LABEL = { phrase: "구 통째로", word: "단어 통째로", syllable: "음절 조각", missing: "영상에 없는 소리 (무음)" };

function renderClips(gen) {
  const box = $("#clips");
  box.replaceChildren();
  box.hidden = false;
  $("#notes-title").textContent = "생성 결과 — 누르면 재생 · 우클릭하면 다른 원본으로 바꾸기";

  const legend = document.createElement("div");
  legend.className = "legend";
  legend.innerHTML =
    '<span><i style="background:#263044"></i>단어·구</span>' +
    '<span><i style="background:#5c5032"></i>음절 조각</span>' +
    '<span><i style="background:#3d2224;border:1px solid var(--error)"></i>없는 소리</span>';
  box.append(legend);

  let lineEl = null;
  let line = -1;
  gen.clips.forEach((clip, i) => {
    if (clip.line !== line) {
      line = clip.line;
      lineEl = document.createElement("div");
      lineEl.className = "clip-line";
      box.append(lineEl);
    }
    const chip = document.createElement("span");
    chip.className = `clip ${clip.kind}`;
    chip.textContent = clip.lyric;
    const src = clip.candidates[clip.choice];
    if (clip.candidates.length > 1) {
      const alt = document.createElement("span");
      alt.className = "alt";
      alt.textContent = `${clip.choice + 1}/${clip.candidates.length}`;
      chip.append(alt);
    }
    chip.title = src
      ? `${KIND_LABEL[clip.kind]} · 원본에서 "${src.said}" (${fmtTime(src.start)})`
      : KIND_LABEL.missing;
    chip.addEventListener("click", () => playClip(i, chip));
    chip.addEventListener("contextmenu", (ev) => {
      ev.preventDefault();
      swapClip(i);
    });
    lineEl.append(chip);
  });
}

async function playClip(index, chip) {
  const item = state.project.generation.timeline.find((t) => t.clip === index);
  if (!item) return;
  if (state.playerSrc !== "preview") await loadPreview();
  playRange(item.start, item.end, chip);
}

async function swapClip(index) {
  const clip = state.project.generation.clips[index];
  if (state.busy || clip.candidates.length < 2) return;
  state.busy = true;
  refreshSteps();
  setStatus("다른 원본으로 바꾸는 중…");
  try {
    setProject(await invoke("swap_clip", { index }));
    state.view = "result";
    renderNotes();
    await loadPreview();
    const chip = $("#clips").querySelectorAll(".clip")[index];
    playClip(index, chip);
  } catch (e) {
    toast(String(e), true);
  } finally {
    setStatus("");
    state.busy = false;
    refreshSteps();
  }
}

// ---------- 인식 결과 (분석 후 선택한 영상의 단어 목록) ----------
let clipStopAt = null;
$("#player").addEventListener("timeupdate", () => {
  const player = $("#player");
  if (clipStopAt !== null && player.currentTime >= clipStopAt) {
    player.pause();
    clipStopAt = null;
    document.querySelector(".playing")?.classList.remove("playing");
  }
});
$("#player").addEventListener("seeking", () => {
  // 사용자가 직접 탐색하면 구간 재생 해제
  if (!$("#player").dataset.clipSeek) clipStopAt = null;
});

function playRange(start, end, el) {
  const player = $("#player");
  document.querySelector(".playing")?.classList.remove("playing");
  el?.classList.add("playing");
  player.dataset.clipSeek = "1";
  player.currentTime = start;
  clipStopAt = end;
  player.play();
  setTimeout(() => delete player.dataset.clipSeek, 50);
}

async function renderTranscript() {
  const box = $("#transcript");
  const asset = state.project?.assets.find((a) => a.id === state.selectedId);
  $("#notes-title").textContent = asset ? `인식 결과 — ${asset.file_name}` : "인식 결과";
  if (!asset) return showNotesEmpty("왼쪽에서 영상을 고르면 인식된 단어가 나타납니다. 단어를 누르면 그 부분만 재생됩니다.");
  if (asset.analysis !== "done") return showNotesEmpty("분석을 마치면 인식된 단어가 나타납니다.");

  let data;
  try {
    data = await invoke("get_transcript", { assetId: asset.id });
  } catch (e) {
    toast(String(e), true);
    return;
  }
  if (state.selectedId !== asset.id || state.view !== "transcript") return; // 그새 화면이 바뀜
  box.hidden = false;
  box.replaceChildren();
  if (data.segments.length === 0) {
    box.textContent = "인식된 말소리가 없습니다.";
    return;
  }
  for (const seg of data.segments) {
    const row = document.createElement("div");
    row.className = "seg";
    const time = document.createElement("span");
    time.className = "time";
    time.textContent = fmtTime(seg.start);
    time.title = "이 문장 재생";
    time.addEventListener("click", () => playRange(seg.start, seg.end));
    const words = document.createElement("div");
    words.className = "words";
    for (const w of seg.words) {
      const chip = document.createElement("span");
      chip.className = "word";
      if (w.prob < 0.5) chip.classList.add("low"); // 인식 신뢰도 낮음
      chip.textContent = w.text;
      chip.title = `${w.start.toFixed(2)}s – ${w.end.toFixed(2)}s · 신뢰도 ${Math.round(w.prob * 100)}%`;
      chip.addEventListener("click", () => playRange(w.start, w.end, chip));
      words.append(chip);
    }
    row.append(time, words);
    box.append(row);
  }
}

async function importVideos() {
  if (state.busy) return;
  const picked = await dialog.open({
    multiple: true,
    title: "영상 불러오기",
    filters: [{ name: "영상", extensions: VIDEO_EXTS }],
  });
  if (!picked || picked.length === 0) return;
  const paths = Array.isArray(picked) ? picked : [picked];

  state.busy = true;
  refreshSteps();
  const unlisten = await listen("import-progress", ({ payload }) => {
    setStatus(payload.done < payload.total
      ? `불러오는 중 ${payload.done + 1}/${payload.total} — ${payload.file}`
      : "");
  });
  try {
    const result = await invoke("import_videos", { paths });
    // 새 영상이 들어오면 setProject가 분석 단계를 미완료로 되돌림
    setProject(result.project);
    if (result.errors.length) toast(result.errors.join("\n"), true);
    else toast(`${paths.length}개 영상을 불러왔습니다`);
  } catch (e) {
    toast(String(e), true);
  } finally {
    unlisten();
    setStatus("");
    state.busy = false;
    refreshSteps();
  }
}

async function removeAsset(id) {
  try {
    setProject(await invoke("remove_asset", { assetId: id }));
    if (state.selectedId === null) {
      $("#player").removeAttribute("src");
      $("#player").load();
      $("#player-empty").hidden = false;
    }
    // 빠진 영상의 조각이 생성 결과에 쓰였을 수 있으므로 생성부터 다시
    if (state.steps.analyze === "done") resetStepsFrom("generate");
  } catch (e) {
    toast(String(e), true);
  }
}

// ---------- 가사 ----------
let lyricsTimer;
$("#lyrics").addEventListener("input", () => {
  clearTimeout(lyricsTimer);
  lyricsTimer = setTimeout(() => invoke("save_lyrics", { lyrics: $("#lyrics").value }).catch(() => {}), 500);
  // 가사가 바뀌면 생성부터 다시
  if (state.steps.generate === "done") resetStepsFrom("generate");
  refreshSteps();
});

// ---------- 단계 버튼 ----------
const ORDER = ["analyze", "generate", "convert", "export"];

function resetStepsFrom(step) {
  for (const s of ORDER.slice(ORDER.indexOf(step))) state.steps[s] = "locked";
  refreshSteps();
}

function prerequisitesMet(step) {
  const hasAssets = (state.project?.assets.length ?? 0) > 0;
  switch (step) {
    case "analyze": return hasAssets && !state.project.assets.every(isFullyAnalyzed);
    case "generate": return state.steps.analyze === "done" && $("#lyrics").value.trim() !== "";
    case "convert": return state.steps.generate === "done";
    case "export": return state.steps.convert === "done";
  }
}

function refreshSteps() {
  for (const step of ORDER) {
    const btn = document.querySelector(`.step[data-step="${step}"]`);
    let status = state.steps[step];
    if (status === "locked" && prerequisitesMet(step)) status = "ready";
    btn.dataset.status = status;
    btn.disabled = state.busy || !(status === "ready" || status === "done" || status === "error");
  }
}

function confirmDisclaimer() {
  const dlg = $("#disclaimer");
  return new Promise((resolve) => {
    const done = (ok) => {
      dlg.close();
      $("#disclaimer-ok").onclick = $("#disclaimer-cancel").onclick = dlg.oncancel = null;
      resolve(ok);
    };
    $("#disclaimer-ok").onclick = () => done(true);
    $("#disclaimer-cancel").onclick = () => done(false);
    dlg.oncancel = () => done(false);
    dlg.showModal();
  });
}

const STEP_LABEL = { analyze: "분석", generate: "생성", convert: "변환", export: "내보내기" };

async function runAnalyze() {
  state.busy = true;
  state.steps.analyze = "working";
  resetStepsFrom("generate");
  $("#btn-cancel").hidden = false;
  $("#btn-cancel").disabled = false;
  setStatus("음성 인식 모델 불러오는 중…");
  const unlisten = await listen("analyze-progress", ({ payload: p }) => {
    const stage = { stt: "1/3 음성 인식", align: "2/3 발음 정렬", check: "3/3 조각 미리 듣기" }[p.stage] ?? p.stage;
    setStatus(`${stage} 중 ${p.index + 1}/${p.total} — ${p.file} ${p.percent}%`);
  });
  try {
    const r = await invoke("analyze");
    setProject(r.project);
    if (r.cancelled) {
      state.steps.analyze = "locked";
      toast("분석을 취소했습니다. 다시 누르면 남은 영상부터 이어서 분석합니다.");
    } else if (r.errors.length) {
      state.steps.analyze = "error";
      toast(r.errors.join("\n"), true);
    } else {
      state.steps.analyze = "done";
      toast("분석 완료");
    }
  } catch (e) {
    state.steps.analyze = "error";
    toast(String(e), true);
  } finally {
    unlisten();
    setStatus("");
    $("#btn-cancel").hidden = true;
    state.busy = false;
    refreshSteps();
    renderAssets();
    renderNotes();
  }
}

async function runGenerate() {
  // 생성할 때마다 저작권 고지 확인
  if (!(await confirmDisclaimer())) return;
  state.busy = true;
  state.steps.generate = "working";
  resetStepsFrom("convert");
  $("#btn-cancel").hidden = false;
  $("#btn-cancel").disabled = false;
  setStatus("가사 조각 찾는 중…");
  const unlisten = await listen("generate-progress", ({ payload: p }) => {
    setStatus(p.stage === "verify"
      ? `생성 중 — 잘 들리는 조각 고르는 중 ${p.done}/${p.total}`
      : `생성 중 — 영상 조각 ${p.done}/${p.total}`);
  });
  try {
    setProject(await invoke("generate", { lyrics: $("#lyrics").value }));
    state.steps.generate = "done";
    state.view = "result";
    renderNotes();
    await loadPreview();
    const missing = state.project.generation.clips.filter((c) => c.kind === "missing").length;
    toast(missing
      ? `생성 완료 — 영상에 없는 소리 ${missing}개는 빨간색(무음)으로 표시됩니다`
      : "생성 완료 — 들어 보고, '음계' 탭에서 블록을 옮겨 노래로 만드세요");
  } catch (e) {
    state.steps.generate = String(e).includes("취소") ? "locked" : "error";
    toast(String(e), true);
  } finally {
    unlisten();
    setStatus("");
    $("#btn-cancel").hidden = true;
    state.busy = false;
    refreshSteps();
  }
}

$("#btn-cancel").addEventListener("click", () => {
  $("#btn-cancel").disabled = true;
  setStatus("취소하는 중…");
  invoke("cancel_job");
});

async function runConvert() {
  state.busy = true;
  state.steps.convert = "working";
  resetStepsFrom("export");
  refreshSteps();
  $("#btn-cancel").hidden = false;
  $("#btn-cancel").disabled = false;
  const unlisten = await listen("convert-progress", ({ payload: p }) => {
    setStatus(p.stage === "sing"
      ? `변환 중 — 노래로 바꾸는 중 ${p.done}/${p.total}`
      : `변환 중 — 영상 만드는 중 ${p.done}/${p.total}`);
  });
  try {
    setProject(await invoke("convert_song"));
    state.steps.convert = "done";
    state.view = "song";
    renderNotes();
    await loadConverted();
    toast("변환 완료 — 재생 버튼으로 들어보세요");
  } catch (e) {
    state.steps.convert = String(e).includes("취소") ? "locked" : "error";
    toast(String(e), true);
  } finally {
    unlisten();
    setStatus("");
    $("#btn-cancel").hidden = true;
    state.busy = false;
    refreshSteps();
  }
}

// ---------- 내보내기 ----------
const EXPORT_FILES = {
  mp4: { ext: "mp4", name: "영상", label: "영상 (MP4)" },
  wav_mix: { ext: "wav", name: "노래", label: "소리 (WAV)" },
  wav_vocal: { ext: "wav", name: "목소리", label: "목소리 (WAV)" },
  midi: { ext: "mid", name: "멜로디", label: "MIDI" },
};

function chooseExportKind() {
  const dlg = $("#export-dlg");
  return new Promise((resolve) => {
    const done = (kind) => {
      dlg.close();
      $("#export-ok").onclick = $("#export-cancel").onclick = dlg.oncancel = null;
      resolve(kind);
    };
    $("#export-ok").onclick = () => done(document.querySelector('input[name="export-kind"]:checked').value);
    $("#export-cancel").onclick = () => done(null);
    dlg.oncancel = () => done(null);
    dlg.showModal();
  });
}

async function runExport() {
  const kind = await chooseExportKind();
  if (!kind) return;
  const f = EXPORT_FILES[kind];
  const dest = await dialog.save({
    title: "내보내기",
    defaultPath: `${state.project.name}_${f.name}.${f.ext}`,
    filters: [{ name: f.label, extensions: [f.ext] }],
  });
  if (!dest) return;
  state.busy = true;
  state.steps.export = "working";
  refreshSteps();
  const unlisten = await listen("export-progress", ({ payload: p }) => setStatus(`내보내는 중 — 고화질 영상 ${p.done}/${p.total}`));
  setStatus("내보내는 중…");
  try {
    await invoke("export", { kind, dest });
    state.steps.export = "done";
    toast(`저장했습니다: ${dest}`);
  } catch (e) {
    state.steps.export = "error";
    toast(String(e), true);
  } finally {
    unlisten();
    setStatus("");
    state.busy = false;
    refreshSteps();
  }
}

async function runStep(step) {
  if (step === "analyze") return runAnalyze();
  if (step === "export") return runExport();
  if (step === "generate") return runGenerate();
  if (step === "convert") return runConvert();

  toast(`${STEP_LABEL[step]}: 알 수 없는 단계`, true);
}

for (const btn of document.querySelectorAll(".step")) {
  btn.addEventListener("click", () => runStep(btn.dataset.step));
}

// ---------- 영역 크기 조절 ----------
const LAYOUT_DEFAULT = { left: 240, right: 300, notes: 300 };
const LAYOUT_KEY = "dcinvox.layout";

function applyLayout(l) {
  const app = $("#app");
  app.style.setProperty("--left-w", `${l.left}px`);
  app.style.setProperty("--right-w", `${l.right}px`);
  app.style.setProperty("--notes-h", `${l.notes}px`);
}

// 창 크기에 맞게 범위 제한 (가운데 영상이 너무 작아지지 않게)
function clampLayout(l) {
  const w = window.innerWidth;
  const h = window.innerHeight;
  const left = Math.max(160, Math.min(l.left, w * 0.4));
  const right = Math.max(200, Math.min(l.right, w - left - 320));
  const notes = Math.max(150, Math.min(l.notes, h - 44 - 56 - 160));
  return { left, right, notes };
}

let layout = { ...LAYOUT_DEFAULT };
try {
  layout = { ...layout, ...JSON.parse(localStorage.getItem(LAYOUT_KEY) || "{}") };
} catch {
  // 저장된 크기를 못 읽으면 기본값
}
applyLayout(clampLayout(layout));
window.addEventListener("resize", () => applyLayout(clampLayout(layout)));

for (const handle of document.querySelectorAll(".splitter")) {
  const which = handle.dataset.split;
  handle.addEventListener("pointerdown", (ev) => {
    ev.preventDefault();
    const start = { x: ev.clientX, y: ev.clientY, ...layout };
    handle.classList.add("dragging");
    document.body.classList.add("resizing");
    const move = (e) => {
      const next = { ...layout };
      if (which === "left") next.left = start.left + (e.clientX - start.x);
      if (which === "right") next.right = start.right - (e.clientX - start.x);
      if (which === "notes") next.notes = start.notes - (e.clientY - start.y);
      layout = clampLayout(next);
      applyLayout(layout);
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      handle.classList.remove("dragging");
      document.body.classList.remove("resizing");
      try {
        localStorage.setItem(LAYOUT_KEY, JSON.stringify(layout));
      } catch {
        // 저장 못 해도 이번 실행 동안은 유지
      }
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  });
  handle.addEventListener("dblclick", () => {
    layout[which] = LAYOUT_DEFAULT[which];
    layout = clampLayout(layout);
    applyLayout(layout);
    try {
      localStorage.setItem(LAYOUT_KEY, JSON.stringify(layout));
    } catch {
      // 무시
    }
  });
}

// ---------- 시작 ----------
$("#btn-open").addEventListener("click", chooseProject);
$("#btn-switch").addEventListener("click", chooseProject);
$("#btn-import").addEventListener("click", importVideos);
