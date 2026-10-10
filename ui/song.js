// 음계 블록 편집기 — "블록을 위아래로 옮기면 음이 바뀐다"만 알면 되게.
// 음은 현재 키의 7음(도레미파솔라시)에만 놓이므로 어떻게 옮겨도 음이 틀리지 않는다.
// app.js의 state·$·invoke·toast·resetStepsFrom·playRange·loadConverted를 사용.

const MAJOR_STEPS = [0, 2, 4, 5, 7, 9, 11];
const TOP_DEG = 9; // 높은 레
const BOTTOM_DEG = -4; // 낮은 파
const ROW_H = 16;
const PX_BEAT = 48;
const LABEL_W = 40;
const LEAD_IN = 0.3; // convert.rs의 LEAD_IN과 같아야 재생 커서가 맞음 (반주가 있으면 반주의 노래 시작 위치)
const songLead = (song) => song.mr ? Math.max(0, song.mr.start - (song.mr.skip ?? 0)) : LEAD_IN;

let selectedNote = null;
let undoStack = [];
let saveTimer = null;
let audioCtx = null;

const curSong = () => state.project?.song;
const clipsOf = () => state.project?.generation?.clips ?? [];

function noteStarts(song) {
  const clips = clipsOf();
  let t = 0;
  return song.notes.map((n, i) => {
    if (i > 0 && clips[song.notes[i - 1].clip]?.line !== clips[n.clip]?.line) t += song.line_rest;
    t += n.gap ?? 0;
    const at = t;
    t += n.beats;
    return at;
  });
}

function noteMidi(song, n) {
  const oct = Math.floor(n.degree / 7);
  const step = ((n.degree % 7) + 7) % 7;
  return 60 + song.key + 12 * oct + MAJOR_STEPS[step] + (n.sharp ? 1 : 0);
}

// 블록을 옮길 때 그 음을 짧게 들려줌 (실제 변환 음은 화자 옥타브에 맞춰짐)
function beep(midi) {
  try {
    audioCtx ??= new AudioContext();
    const osc = audioCtx.createOscillator();
    const gain = audioCtx.createGain();
    osc.type = "triangle";
    osc.frequency.value = 440 * Math.pow(2, (midi - 69) / 12);
    gain.gain.setValueAtTime(0.15, audioCtx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, audioCtx.currentTime + 0.25);
    osc.connect(gain).connect(audioCtx.destination);
    osc.start();
    osc.stop(audioCtx.currentTime + 0.25);
  } catch {
    // 소리 장치가 없어도 편집은 계속
  }
}

function degLabel(d) {
  const name = degNames()[((d % 7) + 7) % 7];
  return d < 0 ? `${name}↓` : d >= 7 ? `${name}↑` : name;
}

// ---------- 저장 / 되돌리기 ----------
function songChanged(prev) {
  if (prev) {
    undoStack.push(prev);
    if (undoStack.length > 200) undoStack.shift();
  }
  // 고친 뒤에는 다시 변환해야 함 (이전 변환 결과의 위치 정보도 무효)
  state.project.conversion = null;
  if (state.steps.convert === "done") resetStepsFrom("convert");
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => invoke("update_song", { song: curSong() }).catch((e) => toast(String(e), true)), 300);
  renderRoll();
}

function undo() {
  const prev = undoStack.pop();
  if (!prev) return;
  state.project.song = prev;
  songChanged(null);
  syncControls();
}

const snapshot = () => structuredClone(curSong());

// ---------- 그리기 ----------
function renderSongEditor() {
  const song = curSong();
  $("#song").hidden = false;
  if (!song) return;
  syncControls();
  renderRoll();
}

function syncControls() {
  const song = curSong();
  if (!song) return;
  const keySel = $("#key");
  if (keySel.dataset.lang !== LANG) {
    keySel.replaceChildren();
    for (let i = 0; i < 12; i++) keySel.add(new Option(keyName(i), i));
    keySel.dataset.lang = LANG;
  }
  keySel.value = song.key;
  $("#bpm").value = song.bpm;
  $("#bpm-val").textContent = Math.round(song.bpm);
  $("#flat").checked = song.flat;
  $("#mr-bar").hidden = !song.mr;
  if (song.mr) {
    const key = keyName(song.mr.tonic, song.mr.minor);
    $("#mr-info").textContent = t("mr.info", { file: song.mr.file, bpm: Math.round(song.mr.detected_bpm), key });
    $("#mr-start").textContent = t("mr.seconds", { s: song.mr.start.toFixed(2) });
    $("#mr-gain").value = song.mr.gain;
  }
  $("#engine").value = song.engine ?? "smooth";
  $("#vibrato").checked = song.vibrato;
}

function renderRoll() {
  const song = curSong();
  const roll = $("#roll");
  if (!song) return;
  const starts = noteStarts(song);
  const totalBeats = song.notes.length ? starts.at(-1) + song.notes.at(-1).beats : 0;
  const inner = document.createElement("div");
  inner.className = "roll-inner";
  inner.style.width = `${LABEL_W + totalBeats * PX_BEAT + 200}px`;
  inner.style.height = `${(TOP_DEG - BOTTOM_DEG + 1) * ROW_H}px`;

  for (let d = TOP_DEG; d >= BOTTOM_DEG; d--) {
    const row = document.createElement("div");
    row.className = "row" + (((d % 7) + 7) % 7 === 0 ? " do" : "");
    row.style.top = `${(TOP_DEG - d) * ROW_H}px`;
    row.style.height = `${ROW_H}px`;
    const label = document.createElement("div");
    label.className = "label";
    label.textContent = degLabel(d);
    row.append(label);
    inner.append(row);
  }

  song.notes.forEach((n, i) => {
    const blk = document.createElement("div");
    blk.className = "blk" + (n.rest ? " rest" : "") + (i === selectedNote ? " sel" : "");
    blk.dataset.i = i;
    const d = n.rest ? 0 : Math.max(BOTTOM_DEG, Math.min(TOP_DEG, n.degree));
    blk.style.left = `${LABEL_W + starts[i] * PX_BEAT + 1}px`;
    blk.style.top = `${(TOP_DEG - d) * ROW_H + 1}px`;
    blk.style.width = `${n.beats * PX_BEAT - 2}px`;
    blk.style.height = `${ROW_H - 2}px`;
    blk.textContent = n.lyric;
    if (n.sharp) blk.insertAdjacentHTML("beforeend", "<sup>♯</sup>");
    const gap = n.gap ? t("block.gap", { gap: n.gap }) : "";
    blk.title = n.rest ? t("block.rest") : t("block.info", { deg: `${degLabel(n.degree)}${n.sharp ? "♯" : ""}`, beats: n.beats, gap });
    inner.append(blk);
  });

  const cursor = document.createElement("div");
  cursor.className = "cursor";
  cursor.id = "roll-cursor";
  cursor.hidden = true;
  inner.append(cursor);

  const { scrollLeft, scrollTop } = roll;
  roll.replaceChildren(inner);
  roll.scrollLeft = scrollLeft;
  roll.scrollTop = scrollTop;
}

// 처음 열 때 블록들이 보이도록 세로 위치 맞추기
function scrollRollToNotes() {
  const song = curSong();
  if (!song?.notes.length) return;
  const degs = song.notes.filter((n) => !n.rest).map((n) => n.degree).sort((a, b) => a - b);
  const mid = degs[Math.floor(degs.length / 2)] ?? 0;
  const roll = $("#roll");
  roll.scrollTop = (TOP_DEG - mid) * ROW_H - roll.clientHeight / 2;
}

// ---------- 마우스: 위아래로 끌면 음, 오른쪽 끝을 끌면 길이 ----------
$("#roll").addEventListener("pointerdown", (ev) => {
  const blk = ev.target.closest(".blk");
  if (!blk) return;
  const i = Number(blk.dataset.i);
  const song = curSong();
  const note = song.notes[i];
  selectedNote = i;
  $("#roll").focus();
  const rect = blk.getBoundingClientRect();
  const resize = ev.clientX > rect.right - 7;
  const prev = snapshot();
  const start = { x: ev.clientX, y: ev.clientY, degree: note.degree, beats: note.beats };
  let changed = false;
  if (!note.rest && !resize) beep(noteMidi(song, note));
  renderRoll();

  const move = (e) => {
    if (resize) {
      const beats = Math.max(0.5, Math.round((start.beats + (e.clientX - start.x) / PX_BEAT) * 2) / 2);
      if (beats !== note.beats) {
        note.beats = beats;
        changed = true;
        renderRoll();
      }
    } else if (!note.rest) {
      const degree = Math.max(BOTTOM_DEG, Math.min(TOP_DEG, start.degree - Math.round((e.clientY - start.y) / ROW_H)));
      if (degree !== note.degree) {
        note.degree = degree;
        changed = true;
        beep(noteMidi(song, note));
        renderRoll();
      }
    }
  };
  const up = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    if (changed) songChanged(prev);
    else playNote(i);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
});

// 블록 누르기만 하면 그 음절 듣기 (변환했으면 노래, 아니면 생성 미리보기)
async function playNote(i) {
  const spans = state.project.conversion;
  if (spans?.[i]) {
    if (state.playerSrc !== "converted") await loadConverted();
    playRange(spans[i].start, spans[i].end);
    return;
  }
  const clip = curSong().notes[i].clip;
  const item = state.project.generation?.timeline.find((t) => t.clip === clip);
  if (!item) return;
  if (state.playerSrc !== "preview") await loadPreview();
  playRange(item.start, item.end);
}

// ---------- 쉼 (고른 블록 앞의 빈 시간) ----------
function changeGap(delta) {
  const song = curSong();
  if (!song || selectedNote === null) return toast(t("block.pickFirst"));
  const n = song.notes[selectedNote];
  const next = Math.max(0, Math.round(((n.gap ?? 0) + delta) * 2) / 2);
  if (next === (n.gap ?? 0)) return;
  const prev = snapshot();
  n.gap = next;
  songChanged(prev);
}
$("#gap-add").addEventListener("click", () => changeGap(0.5));
$("#gap-sub").addEventListener("click", () => changeGap(-0.5));

// ---------- 키보드 ----------
$("#roll").addEventListener("keydown", (ev) => {
  const song = curSong();
  if (!song) return;
  if ((ev.metaKey || ev.ctrlKey) && ev.key.toLowerCase() === "z") {
    ev.preventDefault();
    undo();
    return;
  }
  if (selectedNote === null) return;
  if (ev.key === " " || ev.key === "Backspace") {
    ev.preventDefault();
    changeGap(ev.key === " " ? 0.5 : -0.5);
    return;
  }
  const n = song.notes[selectedNote];
  if (ev.key === "ArrowUp" || ev.key === "ArrowDown") {
    ev.preventDefault();
    if (n.rest) return;
    const next = Math.max(BOTTOM_DEG, Math.min(TOP_DEG, n.degree + (ev.key === "ArrowUp" ? 1 : -1)));
    if (next === n.degree) return;
    const prev = snapshot();
    n.degree = next;
    beep(noteMidi(song, n));
    songChanged(prev);
  } else if (ev.key === "ArrowLeft" || ev.key === "ArrowRight") {
    ev.preventDefault();
    selectedNote = Math.max(0, Math.min(song.notes.length - 1, selectedNote + (ev.key === "ArrowRight" ? 1 : -1)));
    const sn = song.notes[selectedNote];
    if (!sn.rest) beep(noteMidi(song, sn));
    renderRoll();
  }
});

// ---------- 위쪽 조절 막대 ----------
$("#bpm").addEventListener("input", () => {
  $("#bpm-val").textContent = $("#bpm").value;
});
$("#bpm").addEventListener("change", () => {
  const prev = snapshot();
  curSong().bpm = Number($("#bpm").value);
  songChanged(prev);
});
$("#key").addEventListener("change", () => {
  const prev = snapshot();
  curSong().key = Number($("#key").value);
  songChanged(prev);
});
for (const [id, delta] of [["#all-up", 1], ["#all-down", -1]]) {
  $(id).addEventListener("click", () => {
    const prev = snapshot();
    for (const n of curSong().notes) n.degree = Math.max(BOTTOM_DEG, Math.min(TOP_DEG, n.degree + delta));
    songChanged(prev);
  });
}
// 로봇 오토튠 + 반주: 원곡 가사·음절 위치를 찾아 따라 부르기 (처음 한 번은 1~2분, 같은 가사면 다음부터 바로)
async function followOriginal(prev) {
  state.busy = true;
  refreshSteps();
  const unlisten = await listen("follow-progress", ({ payload }) => setStatus(t("follow.progress", { stage: t(`follow.stage.${payload}`) })));
  try {
    state.project.song = await invoke("follow_original");
    undoStack.push(prev);
    state.project.conversion = null;
    if (state.steps.convert === "done") resetStepsFrom("convert");
    syncControls();
    renderRoll();
    scrollRollToNotes();
    const found = curSong().mr?.syllables?.length ?? 0;
    toast(found ? t("follow.found", { n: found }) : t("follow.notFound"));
  } catch (e) {
    toast(String(e), true);
  } finally {
    unlisten();
    setStatus("");
    state.busy = false;
    refreshSteps();
  }
}

for (const btn of document.querySelectorAll("[data-preset]")) {
  btn.addEventListener("click", async () => {
    const prev = snapshot();
    const s = curSong();
    if (btn.dataset.preset === "robot" && s && !s.midi && s.mr) {
      if (state.busy) return;
      return followOriginal(prev);
    }
    try {
      state.project.song = await invoke("apply_preset", { preset: btn.dataset.preset });
      if (btn.dataset.preset === "robot") {
        const s = curSong();
        if (s.midi) toast(t("robot.midi"));
        else if (s.mr?.melody?.length) toast(t("robot.original"));
      }
      undoStack.push(prev);
      state.project.conversion = null;
      if (state.steps.convert === "done") resetStepsFrom("convert");
      syncControls();
      renderRoll();
      scrollRollToNotes();
    } catch (e) {
      toast(String(e), true);
    }
  });
}
const syncAdvToggle = () => ($("#adv-toggle").textContent = t($("#adv").hidden ? "song.adv" : "song.advOpen"));
$("#adv-toggle").addEventListener("click", () => {
  $("#adv").hidden = !$("#adv").hidden;
  syncAdvToggle();
});
syncAdvToggle();
onLanguageChange(syncAdvToggle);
for (const id of ["flat", "vibrato"]) {
  $(`#${id}`).addEventListener("change", () => {
    const prev = snapshot();
    curSong()[id] = $(`#${id}`).checked;
    songChanged(prev);
  });
}
$("#engine").addEventListener("change", () => {
  const prev = snapshot();
  curSong().engine = $("#engine").value;
  songChanged(prev);
});
$("#sharp").addEventListener("click", () => {
  const song = curSong();
  if (selectedNote === null || song.notes[selectedNote].rest) return toast(t("block.pickFirst"));
  const prev = snapshot();
  const n = song.notes[selectedNote];
  n.sharp = !n.sharp;
  beep(noteMidi(song, n));
  songChanged(prev);
});

// ---------- 재생 커서 (변환된 노래 재생 중) ----------
function updateRollCursor() {
  const cursor = document.getElementById("roll-cursor");
  const song = curSong();
  if (!cursor || !song) return;
  const player = $("#player");
  if (state.playerSrc !== "converted" || state.view !== "song") {
    cursor.hidden = true;
    return;
  }
  const beat = (player.currentTime - songLead(song)) / (60 / song.bpm);
  cursor.hidden = beat < 0;
  const x = LABEL_W + beat * PX_BEAT;
  cursor.style.left = `${x}px`;
  const roll = $("#roll");
  if (!player.paused && (x < roll.scrollLeft + LABEL_W || x > roll.scrollLeft + roll.clientWidth - 40)) {
    roll.scrollLeft = x - LABEL_W - 20;
  }
}
(function cursorLoop() {
  updateRollCursor();
  requestAnimationFrame(cursorLoop);
})();

// ---------- 반주(MR) / 멜로디(MIDI) ----------
const AUDIO_EXTS = ["mp3", "wav", "m4a", "aac", "flac", "ogg", "opus", "wma"];

$("#btn-mr").addEventListener("click", async () => {
  if (!curSong()) return toast(t("song.needGenerate"), true);
  const path = await dialog.open({ title: t("dlg.importMr"), filters: [{ name: t("dlg.audioFilter"), extensions: AUDIO_EXTS }] });
  if (!path) return;
  const prev = snapshot();
  setStatus(t("mr.analyzing"));
  try {
    state.project.song = await invoke("import_mr", { path });
    undoStack.push(prev);
    state.project.conversion = null;
    if (state.steps.convert === "done") resetStepsFrom("convert");
    syncControls();
    renderRoll();
    const s = curSong();
    scrollRollToNotes();
    const mel = s.mr?.melody?.length
      ? t("mr.melodyFound", { n: s.mr.melody.length })
      : t("mr.melodyNotFound");
    toast(t("mr.done", { bpm: s.bpm, key: keyName(s.key), mel }));
  } catch (e) {
    toast(String(e), true);
  } finally {
    setStatus("");
  }
});

$("#btn-midi").addEventListener("click", async () => {
  if (!curSong()) return toast(t("song.needGenerate"), true);
  const path = await dialog.open({ title: t("dlg.importMidi"), filters: [{ name: "MIDI", extensions: ["mid", "midi"] }] });
  if (!path) return;
  const prev = snapshot();
  try {
    const r = await invoke("import_midi", { path });
    state.project.song = r.song;
    undoStack.push(prev);
    state.project.conversion = null;
    if (state.steps.convert === "done") resetStepsFrom("convert");
    syncControls();
    renderRoll();
    scrollRollToNotes();
    const extra = r.midi_notes > r.syllables
      ? t("midi.longer", { n: r.midi_notes - r.syllables })
      : r.midi_notes < r.syllables ? t("midi.shorter", { n: r.syllables - r.midi_notes }) : "";
    toast(t("midi.done", { n: r.midi_notes, extra }));
  } catch (e) {
    toast(String(e), true);
  }
});

function shiftMrStart(beats) {
  const s = curSong();
  if (!s?.mr) return;
  const prev = snapshot();
  s.mr.start = Math.max(0, s.mr.start + beats * (60 / s.bpm));
  syncControls();
  songChanged(prev);
}
$("#mr-earlier").addEventListener("click", () => shiftMrStart(-1));
$("#mr-later").addEventListener("click", () => shiftMrStart(1));
$("#mr-gain").addEventListener("change", () => {
  const s = curSong();
  if (!s?.mr) return;
  const prev = snapshot();
  s.mr.gain = Number($("#mr-gain").value);
  songChanged(prev);
});
$("#mr-remove").addEventListener("click", async () => {
  const prev = snapshot();
  try {
    state.project.song = await invoke("remove_mr");
    undoStack.push(prev);
    state.project.conversion = null;
    if (state.steps.convert === "done") resetStepsFrom("convert");
    syncControls();
    renderRoll();
  } catch (e) {
    toast(String(e), true);
  }
});
