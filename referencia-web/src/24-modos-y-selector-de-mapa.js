/* ---------- modos y selector de mapa ---------- */
const CAMPAIGN = ['deposito', 'cantera', 'fundicion', 'azotea'], WAVES_PER_LEVEL = 3;
let gameMode = 'city', levelIdx = 0, pendingLevel = false, cityDirty = false;
try { gameMode = localStorage.getItem('arena-cero-mode') || 'city'; } catch (e) {}
const mapsEl = $('maps');
function lbKey() { return gameMode === 'city' ? 'ciudad' : gameMode === 'campaign' ? 'campana' : mapId; }
const INTRO = {
  city: 'Mundo abierto en el Distrito Sur: siete puestos tomados por soldados, torretas y drones de vigilancia. Elegí el orden, entrá en sigilo o a los tiros. Cada objetivo cambia el mapa: sin antena no hay refuerzos, sin energía se apagan las torretas.',
  waves: 'Sobreviví a oleadas de drones. Desde la oleada 2 entran kamikazes, cada tres un pesado y cada cinco un Coloso. Disparale al núcleo para hacer crítico y elegí una mejora tras cada oleada.'
};
function renderMaps() {
  $('intro').textContent = gameMode === 'city' ? INTRO.city : INTRO.waves;
  if (gameMode === 'city') {
    const m = MAPS.ciudad;
    mapsEl.innerHTML = `<button class="map-b seq" tabindex="-1" aria-pressed="true"><span class="n">${m.name}</span><span class="z">${m.size}</span><span class="d">${m.desc}</span></button>` +
      OUTPOSTS.map(op => `<button class="map-b seq" tabindex="-1"><span class="n"><em>${op.icon}</em> ${op.full}</span><span class="z"></span><span class="d">${op.task}. ${op.effect}.</span></button>`).join('');
    for (const b of $('modes').children) b.setAttribute('aria-pressed', String(b.dataset.mode === gameMode));
    $('maplabel').textContent = 'Objetivos del distrito · en cualquier orden';
    return;
  }
  const camp = gameMode === 'campaign';
  const ids = camp ? CAMPAIGN : Object.keys(MAPS).filter(id => !MAPS[id].city);
  mapsEl.innerHTML = ids.map((id, i) => { const m = MAPS[id];
    return `<button class="map-b${camp ? ' seq' : ''}" data-map="${id}" id="map-${id}" aria-pressed="${!camp && id === mapId}"${camp ? ' tabindex="-1"' : ''}><span class="n">${camp ? `<em>Nivel ${i + 1}</em> ` : ''}${m.name}</span><span class="z">${m.size}</span><span class="d">${m.desc}</span></button>`; }).join('');
  for (const b of $('modes').children) b.setAttribute('aria-pressed', String(b.dataset.mode === gameMode));
  $('maplabel').textContent = camp ? `Recorrido · cambia cada ${WAVES_PER_LEVEL} oleadas` : 'Mapa';
}
mapsEl.addEventListener('click', e => { const b = e.target.closest('[data-map]'); if (b && gameMode === 'single' && state !== 'paused' && state !== 'playing') selectMap(b.dataset.map); });
$('modes').addEventListener('click', e => { const b = e.target.closest('[data-mode]'); if (b && state !== 'paused' && state !== 'playing') setMode(b.dataset.mode); });
function setMode(m) {
  gameMode = m; try { localStorage.setItem('arena-cero-mode', m); } catch (e) {}
  selectMap(m === 'city' ? 'ciudad' : m === 'campaign' ? CAMPAIGN[0] : savedMapId());
}
function showBest() { $('best').textContent = best ? `Récord en ${gameMode === 'city' ? 'Distrito Sur' : gameMode === 'campaign' ? 'oleadas' : MAP.name}: ${best.toLocaleString('es-AR')}` : ''; }
function selectMap(id) {
  if (!MAPS[id]) id = 'deposito';
  buildMap(id); cityDirty = false; resetGame();
  if (gameMode === 'single') try { localStorage.setItem('arena-cero-map', id); } catch (e) {}
  try { best = +localStorage.getItem('arena-cero-best-' + lbKey()) || 0; } catch (e) { best = 0; }
  renderMaps(); showBest();
  $('lbmap').textContent = gameMode === 'city' ? 'Distrito Sur' : gameMode === 'campaign' ? 'Oleadas' : MAP.name; subscribeLB();
}
function savedMapId() { try { return localStorage.getItem('arena-cero-map') || 'deposito'; } catch (e) { return 'deposito'; } }
function nextLevel() {
  levelIdx++;
  buildMap(CAMPAIGN[levelIdx % CAMPAIGN.length]);
  while (shots.length) removeShot(shots.length - 1);
  for (const o of orbs) scene.remove(o.m); orbs.length = 0;
  for (const n of nades) scene.remove(n.m); nades.length = 0;
  releaseHook(false);
  P.pos.set(MAP.start[0], EYE, MAP.start[1]); P.vel.set(0, 0, 0); P.yaw = Math.atan2(MAP.start[0], MAP.start[1]); P.pitch = 0;
  targetsDirty = true; intermission = 3.5;
  banner(`NIVEL ${levelIdx + 1} · ${MAP.name.toUpperCase()}`, MAP.desc); sfx.perk();
}

