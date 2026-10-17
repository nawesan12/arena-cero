/* ---------- audio ---------- */
let ac = null, master = null, noiseBuf = null;
function initAudio() {
  if (ac) { ac.resume && ac.resume(); return; }
  try {
    ac = new (window.AudioContext || window.webkitAudioContext)();
    master = ac.createGain(); master.gain.value = .45; master.connect(ac.destination);
    noiseBuf = ac.createBuffer(1, ac.sampleRate * 1.2, ac.sampleRate);
    const d = noiseBuf.getChannelData(0); for (let i = 0; i < d.length; i++) d[i] = Math.random()*2-1;
  } catch (e) { ac = null; }
}
function env(g, t, a, len) { g.gain.setValueAtTime(a, t); g.gain.exponentialRampToValueAtTime(.0001, t + len); }
function tone(type, f0, f1, len, vol) {
  if (!ac) return; const t = ac.currentTime, o = ac.createOscillator(), g = ac.createGain();
  o.type = type; o.frequency.setValueAtTime(f0, t); o.frequency.exponentialRampToValueAtTime(Math.max(f1, 1), t + len);
  env(g, t, vol, len); o.connect(g).connect(master); o.start(t); o.stop(t + len + .02);
}
function noise(freq, q, len, vol, type = 'bandpass') {
  if (!ac) return; const t = ac.currentTime, s = ac.createBufferSource(), f = ac.createBiquadFilter(), g = ac.createGain();
  s.buffer = noiseBuf; f.type = type; f.frequency.value = freq; f.Q.value = q; env(g, t, vol, len);
  s.connect(f).connect(g).connect(master); s.start(t); s.stop(t + len + .02);
}
const sfx = {
  shot(w) {
    if (w === 0) { noise(1800, .8, .11, .5); tone('triangle', 170, 45, .1, .35); }
    else if (w === 1) { noise(900, .5, .28, .8); tone('triangle', 120, 30, .2, .5); }
    else { tone('sawtooth', 2200, 180, .35, .25); noise(4000, 1, .2, .3); tone('sine', 90, 30, .3, .5); }
  },
  siren() { tone('sawtooth', 520, 880, .45, .12); setTimeout(() => tone('sawtooth', 880, 520, .45, .12), 450); setTimeout(() => tone('sawtooth', 520, 880, .45, .1), 900); },
  whiz() { noise(3000, 4, .09, .06); },
  eshot(d, big) { const v = Math.max(.02, (big ? .5 : .25) - d * .005); noise(big ? 900 : 1500, .7, big ? .25 : .09, v); tone('triangle', big ? 120 : 200, 60, .08, v * .6); },
  alertBeep() { tone('square', 1200, 1200, .06, .08); setTimeout(() => tone('square', 1500, 1500, .06, .08), 90); },
  tag() { tone('sine', 1800, 2400, .06, .07); },
  takedown() { noise(500, .8, .12, .3, 'lowpass'); tone('sine', 180, 60, .15, .3); },
  plant() { tone('square', 700, 700, .05, .08); setTimeout(() => tone('square', 900, 900, .05, .08), 120); },
  objective() { tone('sine', 520, 520, .18, .2); setTimeout(() => tone('sine', 660, 660, .18, .2), 160); setTimeout(() => tone('sine', 880, 880, .3, .22), 320); },
  ricochet() { tone('sine', 2600, 1200, .08, .06); },
  hit() { tone('square', 950, 1500, .05, .07); },
  crit() { tone('square', 1500, 2400, .07, .1); tone('sine', 3000, 3000, .04, .05); },
  hook() { noise(3200, .8, .18, .25); tone('sawtooth', 900, 260, .12, .12); },
  hookMiss() { tone('square', 500, 180, .1, .06); },
  launch() { tone('square', 240, 80, .16, .2); noise(700, .6, .22, .4); },
  dash() { noise(1500, .5, .25, .3); tone('sine', 200, 700, .18, .2); },
  perk() { tone('sine', 400, 800, .3, .2); setTimeout(() => tone('sine', 600, 1200, .3, .18), 120); },
  boss() { tone('sawtooth', 220, 60, .5, .3); noise(400, .6, .4, .3, 'lowpass'); },
  kill() { noise(500, .6, .5, .6, 'lowpass'); tone('sine', 140, 28, .45, .5); },
  boom() { noise(260, .5, 1, 1, 'lowpass'); tone('sine', 80, 20, .8, .8); },
  enemy(d) { tone('sawtooth', 640, 180, .16, Math.max(.02, .14 - d * .004)); },
  hurt() { tone('sine', 110, 40, .25, .6); noise(300, 1, .15, .3, 'lowpass'); },
  reload() { tone('square', 300, 280, .03, .1); setTimeout(() => tone('square', 520, 500, .04, .12), 900); },
  pickup() { tone('sine', 520, 1300, .22, .25); },
  empty() { tone('square', 200, 190, .03, .08); },
  aim() { noise(2400, 2, .05, .06); },
  swap() { tone('square', 400, 600, .05, .08); },
  djump() { tone('sine', 300, 900, .14, .18); noise(2600, 1, .12, .12); },
  slide() { noise(700, .4, .45, .25, 'lowpass'); },
  toss() { noise(1200, 1, .12, .15); },
  beep() { tone('sine', 1400, 1400, .04, .08); }
};

