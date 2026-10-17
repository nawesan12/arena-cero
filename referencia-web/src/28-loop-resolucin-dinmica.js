/* ---------- loop + resolución dinámica ---------- */
function resize() { const w = innerWidth, h = innerHeight; renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix(); gunCam.aspect = w / h; gunCam.updateProjectionMatrix(); }
addEventListener('resize', resize); resize();

const clock = new THREE.Clock();
let time = 0, ema = 1/60, prTimer = 0, fpsTimer = 0, frames = 0;
function adaptResolution(dt) {
  ema += (dt - ema) * .05;
  prTimer += dt; fpsTimer += dt; frames++;
  if (fpsTimer >= .5) { hud.fps.textContent = state === 'playing' ? `${Math.round(frames / fpsTimer)} FPS` : ''; fpsTimer = 0; frames = 0; }
  if (prTimer < 1.5) return; prTimer = 0;
  let next = pixelRatio;
  if (ema > 1/48 && pixelRatio > .6) next = Math.max(.6, pixelRatio - .15);
  else if (ema < 1/58 && pixelRatio < MAX_PR) next = Math.min(MAX_PR, pixelRatio + .1);
  if (next !== pixelRatio) { pixelRatio = next; renderer.setPixelRatio(pixelRatio); resize(); }
}
function setQuality(q) {
  if (!QUALITY[q]) return;
  quality = q; Q = QUALITY[q]; try { localStorage.setItem('arena-cero-q', q); } catch (e) {}
  MAX_PR = Math.min(devicePixelRatio, Q.pr); pixelRatio = MAX_PR; renderer.setPixelRatio(pixelRatio);
  if (sun.shadow.map) { sun.shadow.map.dispose(); sun.shadow.map = null; }
  sun.shadow.mapSize.set(Q.shadow, Q.shadow); renderer.shadowMap.needsUpdate = true;
  camera.far = Q.far; camera.updateProjectionMatrix();
  if (MAP) { scene.fog.far = Math.min(MAP.fog[1], Q.far - 10); scene.fog.near = Math.min(MAP.fog[0], scene.fog.far * .4); }
  resize();
  for (const b of $('qual').querySelectorAll('button')) b.setAttribute('aria-pressed', String(b.dataset.q === q));
}
$('qual').addEventListener('click', e => { const b = e.target.closest('button[data-q]'); if (b) setQuality(b.dataset.q); });
let lastFrame = 0;
function frame(now = performance.now()) {
  // tope de FPS: en menús 30, en juego según la calidad
  const cap = state === 'playing' ? Q.fps : 30;
  if (cap && now - lastFrame < 1000 / cap - 1) { requestAnimationFrame(frame); return; }
  lastFrame = now;
  const dt = Math.min(clock.getDelta(), .05); time += dt;
  if (state === 'playing') { update(dt, time); updateHud(dt); }
  else if (state === 'menu') {
    camera.position.set(Math.sin(time*.06)*A*.78, A*.32, Math.cos(time*.06)*A*.78);
    camera.lookAt(0, 1, 0);
  }
  gun.visible = state !== 'menu';
  updateFx(dt);
  updateNums(dt);
  updateMarkers();
  const showMap = mapOpen && state === 'playing' && isCity();
  if (bigmap.hidden === showMap) bigmap.hidden = !showMap;
  if (showMap) drawBigMap();
  if (state === 'playing') drawHook(dt);
  music.level = musicLevel();
  renderer.clear();
  renderer.render(scene, camera);
  if (gun.visible) { renderer.clearDepth(); renderer.render(gunScene, gunCam); }
  if (state === 'playing') drawRadar();
  adaptResolution(dt);
  requestAnimationFrame(frame);
}
setQuality(quality);
selectMap(gameMode === 'city' ? 'ciudad' : gameMode === 'campaign' ? CAMPAIGN[0] : savedMapId());
updateHud(0);
frame();
})();
