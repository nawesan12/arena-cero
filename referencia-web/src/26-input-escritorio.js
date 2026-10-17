/* ---------- input escritorio ---------- */
document.addEventListener('pointerlockchange', () => {
  if (document.pointerLockElement) lockedOnce = true;
  else if (state === 'playing' && lockedOnce) { state = 'paused'; firing = aiming = false; showOverlay('pause'); }
});
addEventListener('mousemove', e => {
  if (state !== 'playing' || isTouch) return;
  if (Math.abs(e.movementX) > 180 || Math.abs(e.movementY) > 180) return; // picos al tomar el puntero
  const s = .0022 * sens * (camera.fov / BASE_FOV);
  P.yaw -= e.movementX * s; P.pitch -= e.movementY * s;
  P.swayX = Math.max(-.06, Math.min(.06, P.swayX + e.movementX * .00012));
  P.swayY = Math.max(-.05, Math.min(.05, P.swayY + e.movementY * .0001));
  P.pitch = Math.max(-1.5, Math.min(1.5, P.pitch));
});
addEventListener('contextmenu', e => e.preventDefault());
renderer.domElement.addEventListener('mousedown', e => {
  if (state !== 'playing') return;
  if (!document.pointerLockElement && lockedOnce) return;
  if (e.button === 0) firing = true;
  if (e.button === 2) aiming = true;
  if (e.button === 1) { e.preventDefault(); tryTag(); }
});
addEventListener('mouseup', e => { if (e.button === 0) firing = false; if (e.button === 2) aiming = false; });
addEventListener('wheel', e => { if (state === 'playing') switchWeapon(P.w + (e.deltaY > 0 ? 1 : -1)); }, { passive: true });
addEventListener('keydown', e => {
  keys[e.code] = true;
  if (state === 'playing') {
    if (e.code === 'KeyR') startReload();
    if (e.code === 'KeyG') throwNade();
    if (e.code === 'KeyQ') P.dashReq = true;
    if (e.code === 'KeyT') tryTag();
    if (e.code === 'Tab') { e.preventDefault(); mapOpen = true; }
    if ((e.code === 'ControlLeft' || e.code === 'ControlRight' || e.code === 'KeyE') && !e.repeat) { e.preventDefault(); fireHook(); }
    if (e.code === 'Digit1') switchWeapon(0, true);
    if (e.code === 'Digit2') switchWeapon(1, true);
    if (e.code === 'Digit3') switchWeapon(2, true);
    if (e.code === 'Digit4') switchWeapon(3, true);
  }
  if (state === 'perk' && /^Digit[123]$/.test(e.code)) { pickPerk(+e.code.slice(5) - 1); return; }
  if (e.code === 'KeyM') setMusicVol(musicVol > 0 ? 0 : .6);
  if (e.code === 'Space') { e.preventDefault(); if (!e.repeat && state === 'playing') keys.jumpT = true; }
  if (e.code === 'Enter' && state !== 'playing') begin();
});
addEventListener('keyup', e => { keys[e.code] = false; if (e.code === 'Tab') mapOpen = false; });
addEventListener('blur', () => { for (const k in keys) keys[k] = false; firing = aiming = false; });
document.addEventListener('visibilitychange', () => { if (document.hidden && state === 'playing' && isTouch) { state = 'paused'; firing = false; showOverlay('pause'); } });

