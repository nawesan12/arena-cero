/* ---------- música dinámica ---------- */
let musicVol = .6; try { const v = localStorage.getItem('arena-cero-music'); if (v !== null) musicVol = +v; } catch (e) {}
const music = { gain: null, timer: null, next: 0, step: 0, level: 0 };
const mtof = m => 440 * Math.pow(2, (m - 69) / 12);
const PROG = [[45, 48, 52], [41, 45, 48], [43, 47, 50], [40, 44, 47]]; // La m · Fa · Sol · Mi
function mNote(type, freq, t, dur, vol, cutoff) {
  const o = ac.createOscillator(), g = ac.createGain(); o.type = type; o.frequency.setValueAtTime(freq, t);
  g.gain.setValueAtTime(0, t); g.gain.linearRampToValueAtTime(vol, t + .008); g.gain.exponentialRampToValueAtTime(.0001, t + dur);
  let node = o;
  if (cutoff) { const f = ac.createBiquadFilter(); f.type = 'lowpass'; f.frequency.value = cutoff; o.connect(f); node = f; }
  node.connect(g).connect(music.gain); o.start(t); o.stop(t + dur + .02);
}
function mKick(t) {
  const o = ac.createOscillator(), g = ac.createGain();
  o.frequency.setValueAtTime(140, t); o.frequency.exponentialRampToValueAtTime(42, t + .18);
  g.gain.setValueAtTime(.55, t); g.gain.exponentialRampToValueAtTime(.0001, t + .25);
  o.connect(g).connect(music.gain); o.start(t); o.stop(t + .27);
}
function mNoise(t, dur, vol, freq, type) {
  const src = ac.createBufferSource(), f = ac.createBiquadFilter(), g = ac.createGain();
  src.buffer = noiseBuf; f.type = type; f.frequency.value = freq;
  g.gain.setValueAtTime(vol, t); g.gain.exponentialRampToValueAtTime(.0001, t + dur);
  src.connect(f).connect(g).connect(music.gain); src.start(t, Math.random()*.5); src.stop(t + dur + .02);
}
function musicStep(st, t) {
  const L = music.level, ch = PROG[Math.floor(st / 16) % 4], s16 = st % 16;
  if (s16 === 0) ch.forEach(n => mNote('triangle', mtof(n + 12), t, L ? 1.8 : 2.4, L ? .03 : .05));
  if (L === 0) return;
  if (s16 % 4 === 0) mKick(t);
  if (s16 % 2 === 0) mNote('sawtooth', mtof(ch[0] - 12 + (s16 % 8 === 6 ? 12 : 0)), t, .14, .15, 480 + L*200);
  if (L >= 2) { if (s16 % 2 === 1) mNoise(t, .03, .07, 7000, 'highpass'); if (s16 % 8 === 4) mNoise(t, .12, .2, 1800, 'bandpass'); }
  if (L >= 3 && s16 % 2 === 0) mNote('square', mtof(ch[(s16 / 2) % 3] + 24), t, .1, .04, 2600);
}
function startMusic() {
  if (!ac || music.timer) return;
  music.gain = ac.createGain(); music.gain.gain.value = musicVol * .55; music.gain.connect(ac.destination);
  music.next = ac.currentTime + .1; music.step = 0;
  const sixteenth = 60 / 118 / 4;
  music.timer = setInterval(() => {
    if (music.next < ac.currentTime - .25) music.next = ac.currentTime + .05;
    while (music.next < ac.currentTime + .15) { musicStep(music.step, music.next); music.next += sixteenth; music.step = (music.step + 1) % 64; }
  }, 30);
}
function setMusicVol(v) {
  musicVol = v; if (music.gain) music.gain.gain.setTargetAtTime(v * .55, ac.currentTime, .05);
  try { localStorage.setItem('arena-cero-music', v); } catch (e) {}
  const vi = $('vol'), vo = $('volv'); if (vi) { vi.value = v; vo.textContent = Math.round(v * 100); }
}
function musicLevel() {
  if (isCity()) { if (state !== 'playing') return 0; const al = cityAlert(); const cmd = OUTPOSTS.find(o => o.id === 'mando'); if (cmd && cmd.commander && cmd.commander.hp > 0 && cmd.commander.st === 'combat') return 3; return al === 2 ? 2 : al; }
  if (state !== 'playing' || (!enemies.length && !spawnQueue.length)) return 0;
  if (bossRef) return 3;
  return (combo >= 3 || P.hp < P.maxHp * .35 || enemies.length > 7) ? 2 : 1;
}

