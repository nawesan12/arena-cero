/* ---------- oleadas ---------- */
let spawnTimer = 0, bannerT = 0;
function startWave() {
  wave++;
  const bossWave = wave % 5 === 0;
  const n = Math.round((3 + wave*1.5) * (bossWave ? .5 : 1)), heavies = bossWave ? 0 : Math.floor(wave/3), kamis = wave >= 2 ? Math.floor(wave * .5) : 0;
  spawnQueue = [];
  for (let i = 0; i < n; i++) spawnQueue.push('d');
  for (let i = 0; i < kamis; i++) spawnQueue.splice(1 + Math.floor(Math.random()*spawnQueue.length), 0, 'k');
  for (let i = 0; i < heavies; i++) spawnQueue.splice(Math.floor(Math.random()*spawnQueue.length), 0, 'h');
  if (bossWave) spawnQueue.unshift('b');
  spawnTimer = .4;
  const extra = [kamis && `${kamis} kamikaze${kamis > 1 ? 's' : ''}`, heavies && `${heavies} pesado${heavies > 1 ? 's' : ''}`].filter(Boolean);
  if (bossWave) banner(`OLEADA ${wave} · COLOSO`, ['Jefe entrante', `${n} drones`, ...extra].join(' · '));
  else banner(`OLEADA ${wave}`, [`${n} drones`, ...extra].join(' · '));
}
function banner(t, s) { $('bt').textContent = t; $('bs').textContent = s; hud.banner.style.opacity = 1; bannerT = 2.2; }

