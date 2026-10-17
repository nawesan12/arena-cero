/* ---------- input táctil ---------- */
if (isTouch) {
  const tz = $('touch'), joy = $('joy'), knob = joy.firstElementChild;
  let joyId = null, joyO = null, lookId = null, lookX = 0, lookY = 0;
  tz.addEventListener('pointerdown', e => {
    if (state !== 'playing') return;
    const t = e.target;
    if (t.id === 'tfire') { firing = true; t.setPointerCapture(e.pointerId); return; }
    if (t.id === 'tjump') { keys.jumpT = true; return; }
    if (t.id === 'trel') { startReload(); return; }
    if (t.id === 'taim') { aimToggle = !aimToggle; t.classList.toggle('on', aimToggle); return; }
    if (t.id === 'tcrouch') { crouchToggle = !crouchToggle; t.classList.toggle('on', crouchToggle); return; }
    if (t.id === 'tweap') { switchWeapon(P.w + 1); return; }
    if (t.id === 'tnade') { throwNade(); return; }
    if (t.id === 'tdash') { P.dashReq = true; return; }
    if (t.id === 'thook') { fireHook(); return; }
    if (t.id === 'tuse') { keys.KeyF = true; t.setPointerCapture(e.pointerId); return; }
    if (t.id === 'tmap') { mapOpen = !mapOpen; t.classList.toggle('on', mapOpen); return; }
    if (t.id === 'ttag') { tryTag(); return; }
    if (e.clientX < innerWidth*.45 && joyId === null) {
      joyId = e.pointerId; joyO = { x: e.clientX, y: e.clientY };
      joy.style.display = 'block'; joy.style.left = (e.clientX - 60) + 'px'; joy.style.top = (e.clientY - 60) + 'px';
    } else if (lookId === null) { lookId = e.pointerId; lookX = e.clientX; lookY = e.clientY; }
  });
  tz.addEventListener('pointermove', e => {
    if (e.pointerId === joyId) {
      let dx = (e.clientX - joyO.x) / 50, dy = (e.clientY - joyO.y) / 50; const l = Math.hypot(dx, dy); if (l > 1) { dx /= l; dy /= l; }
      touchMove.x = dx; touchMove.y = -dy; knob.style.transform = `translate(${dx*40}px,${dy*40}px)`;
    } else if (e.pointerId === lookId) {
      const s = .006 * sens * (camera.fov / BASE_FOV);
      P.yaw -= (e.clientX - lookX) * s; P.pitch -= (e.clientY - lookY) * s;
      P.pitch = Math.max(-1.5, Math.min(1.5, P.pitch)); lookX = e.clientX; lookY = e.clientY;
    }
  });
  const end = e => {
    if (e.target.id === 'tuse') keys.KeyF = false;
    if (e.target.id === 'tfire') firing = false;
    if (e.pointerId === joyId) { joyId = null; touchMove.x = touchMove.y = 0; joy.style.display = 'none'; knob.style.transform = ''; }
    if (e.pointerId === lookId) lookId = null;
  };
  tz.addEventListener('pointerup', end); tz.addEventListener('pointercancel', end);
}

