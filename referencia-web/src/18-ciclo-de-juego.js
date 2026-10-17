/* ---------- ciclo de juego ---------- */
function resetGame() {
  for (const e of enemies) scene.remove(e.g); enemies.length = 0; targetsDirty = true;
  while (shots.length) removeShot(shots.length - 1);
  for (const o of orbs) scene.remove(o.m); orbs.length = 0;
  for (const n of nades) scene.remove(n.m); nades.length = 0;
  Object.assign(P, { hp: 100, ammo: WEAPONS.map(w => w.mag), w: 0, reload: 0, fireCd: 0, yaw: 0, pitch: 0, onGround: true, recoil: 0, aimT: 0,
    crouchT: 0, slide: 0, slideCd: 0, swapT: 0, nades: 2, nadeCd: 0, shake: 0, jumps: 0 });
  Object.assign(P, BASE_STATS()); perksTaken = []; bossRef = null; $('perks').hidden = true;
  applyGunModel(0);
  P.pos.set(MAP.start[0], EYE, MAP.start[1]); P.vel.set(0, 0, 0); P.yaw = Math.atan2(MAP.start[0], MAP.start[1]);
  if (cable) { cable.visible = spike.visible = false; }
  aiming = aimToggle = crouchToggle = false; semiLock = false;
  $('taim').classList.remove('on'); $('tcrouch').classList.remove('on');
  levelIdx = 0; pendingLevel = false;
  wave = 0; score = 0; kills = 0; combo = 0; comboT = 0; spawnQueue = []; intermission = 1.2; hurtFlash = 0;
  document.body.classList.toggle('city', isCity());
  if (isCity()) initCity(); else P.unlocked = null;
  last.slotsKey = null;
}
let dmgT = 0;
function hurt(d, from) {
  if (d <= 0 || state !== 'playing') return;
  P.hp = Math.max(0, P.hp - d); P.lastHurt = 0; hurtFlash = Math.min(1, hurtFlash + .55); sfx.hurt();
  P.shake = Math.min(1, P.shake + .25);
  if (from) {
    const dx = from.x - P.pos.x, dz = from.z - P.pos.z, sy = Math.sin(P.yaw), cy = Math.cos(P.yaw);
    const fwd = -dx*sy - dz*cy, rgt = dx*cy - dz*sy;
    hud.dmgdir.style.transform = `rotate(${Math.atan2(rgt, fwd)}rad)`; dmgT = 1;
  }
  if (P.hp <= 0) gameOver();
}
function gameOver() {
  if (isCity() && cityState.lives > 1 && state === 'playing') { cityRespawn(); return; }
  state = 'over'; firing = false; aiming = false;
  const isBest = score > best;
  if (isBest) { best = score; try { localStorage.setItem('arena-cero-best-' + lbKey(), best); } catch (e) {} }
  if (document.pointerLockElement) document.exitPointerLock();
  releaseHook(false);
  showOverlay('over', isBest);
  submitScore();
}

let lastFov = BASE_FOV;
function update(dt, t) {
  /* agacharse / deslizarse */
  const wantCrouch = keys.KeyC || crouchToggle;
  const hs0 = Math.hypot(P.vel.x, P.vel.z);
  if (wantCrouch && !P.wasCrouch && P.onGround && hs0 > 8 && P.slideCd <= 0) {
    P.slide = .7; P.slideCd = 1; sfx.slide();
    const k = 14 / hs0; P.vel.x *= k; P.vel.z *= k;
  }
  P.wasCrouch = wantCrouch;
  P.slideCd -= dt;
  if (P.slide > 0) P.slide -= dt;
  P.crouchT += ((wantCrouch || P.slide > 0 ? 1 : 0) - P.crouchT) * (1 - Math.exp(-14 * dt));

  /* apuntado */
  const wantAim = (aiming || aimToggle) && P.reload <= 0 && P.swapT <= 0 && P.slide <= 0;
  const prevAim = P.aimT;
  P.aimT += ((wantAim ? 1 : 0) - P.aimT) * (1 - Math.exp(-14 * dt));
  if (P.aimT < .001) P.aimT = 0;
  if (prevAim < .5 && P.aimT >= .5) sfx.aim();
  const targetFov = (P.w === 2 ? AIM_FOV - 24 : AIM_FOV);
  const fov = BASE_FOV + (targetFov - BASE_FOV) * P.aimT + (P.slide > 0 ? 6 : 0) + (P.dashT > 0 ? 10 : 0);
  if (Math.abs(fov - lastFov) > .01) { camera.fov = lastFov += (fov - lastFov) * Math.min(1, dt * 20); camera.updateProjectionMatrix(); }
  const gf = GUN_FOV + (GUN_FOV_AIM - GUN_FOV) * P.aimT;
  if (Math.abs(gf - gunCam.fov) > .01) { gunCam.fov = gf; gunCam.updateProjectionMatrix(); }

  /* movimiento */
  const f = (keys.KeyW ? 1 : 0) - (keys.KeyS ? 1 : 0) + touchMove.y;
  const r = (keys.KeyD ? 1 : 0) - (keys.KeyA ? 1 : 0) + touchMove.x;
  const sy = Math.sin(P.yaw), cy = Math.cos(P.yaw);
  let wx = -sy*f + cy*r, wz = -cy*f - sy*r;
  const wl = Math.hypot(wx, wz); if (wl > 1) { wx /= wl; wz /= wl; }
  const sprint = (keys.ShiftLeft || keys.ShiftRight) && P.aimT < .3 && P.crouchT < .3;
  const spd = (sprint ? 10 : 7) * (1 - P.aimT * .45) * (1 - P.crouchT * .5) * P.speedMul;
  P.dashCd -= dt;
  if (P.dashReq) {
    P.dashReq = false;
    if (P.dashCd <= 0) {
      let dx = wx, dz = wz; if (Math.hypot(dx, dz) < .1) { dx = -sy; dz = -cy; }
      const l = Math.hypot(dx, dz); P.vel.x = dx/l*24; P.vel.z = dz/l*24; P.vel.y = Math.max(P.vel.y, 1);
      P.dashT = .16; P.dashCd = 3 * P.dashCdMul; P.slide = 0; P.shake = Math.min(1, P.shake + .2);
      _a.set(P.pos.x, P.pos.y - 1, P.pos.z); burst(_a, 0x5fd8ff, 18, 3, .35); sfx.dash();
    }
  }
  if (P.dashT > 0) { P.dashT -= dt; if (P.dashT <= 0) { P.vel.x *= .45; P.vel.z *= .45; } }
  else if (P.slide > 0 && P.onGround) {
    const fr = 1 - Math.exp(-1.6 * dt); P.vel.x -= P.vel.x * fr; P.vel.z -= P.vel.z * fr;
    if (Math.random() < .5) { _a.set(P.pos.x, .05, P.pos.z); burst(_a, 0x3a4246, 1, 2, .4); }
  } else if (P.onGround && !P.hook) {
    const k = 1 - Math.exp(-12 * dt);
    P.vel.x += (wx*spd - P.vel.x) * k; P.vel.z += (wz*spd - P.vel.z) * k;
  } else {
    // control aéreo: suma en la dirección del input sin frenar el impulso del gancho o el dash
    const along = P.vel.x*wx + P.vel.z*wz;
    if (along < spd) { P.vel.x += wx*spd*2.4*dt; P.vel.z += wz*spd*2.4*dt; }
    const dr = Math.exp(-(P.hook ? .1 : .6) * dt); P.vel.x *= dr; P.vel.z *= dr;
  }
  updateHook(dt);
  if (keys.jumpT) {
    if (P.hook) releaseHook(true);
    else if (P.onGround) { P.vel.y = 8.2; P.onGround = false; P.slide = 0; P.jumps = 1; }
    else if (P.jumps < P.maxJumps) { // doble salto: impulso en aire hacia donde apunta el input
      P.vel.y = 7.6; P.jumps++;
      if (wl > .1) { P.vel.x = wx*spd*1.1; P.vel.z = wz*spd*1.1; }
      _a.set(P.pos.x, P.pos.y - EYE, P.pos.z); burst(_a, 0x5fd8ff, 16, 4, .4); sfx.djump();
    }
  }
  keys.jumpT = false;
  P.vel.y -= 24 * dt;
  let feet = P.pos.y - EYE;
  const nSub = Math.min(8, Math.max(1, Math.ceil(Math.hypot(P.vel.x, P.vel.z) * dt / .3)));
  for (let si = 0; si < nSub; si++) { P.pos.x += P.vel.x * dt / nSub; P.pos.z += P.vel.z * dt / nSub; collideXZ(P.pos, R, feet, 1.8 - P.crouchT * .7); }
  const feet0 = feet; feet += P.vel.y * dt;
  if (P.vel.y > 0) { // techo: no atravesar losas al saltar o con el gancho
    const hh = 1.8 - P.crouchT * .7, c = ceilingAt(P.pos.x, P.pos.z, feet0 + hh, feet + hh);
    if (c !== null) { feet = c - hh - .01; P.vel.y = 0; }
  }
  const g = groundAt(P.pos.x, P.pos.z, P.pos.y - EYE);
  if (feet <= g) { feet = g; P.vel.y = 0; P.onGround = true; P.jumps = 0; } else if (feet > g + .05) P.onGround = false;
  P.pos.y = feet + EYE;
  if (feet < .2) for (const zn of zones) if (Math.abs(P.pos.x - zn.x) < zn.w/2 && Math.abs(P.pos.z - zn.z) < zn.d/2) {
    P.zoneAcc += zn.dps * dt;
    if (Math.random() < .4) { _a.set(P.pos.x, .1, P.pos.z); burst(_a, 0xff8030, 2, 3, .4); }
    if (P.zoneAcc >= 4) { P.zoneAcc -= 4; hurt(4); if (state !== 'playing') return; }
  }
  P.pos.x = Math.max(-LIM, Math.min(LIM, P.pos.x)); P.pos.z = Math.max(-LIM, Math.min(LIM, P.pos.z));

  /* arma */
  P.fireCd -= dt; P.nadeCd -= dt;
  if (P.swapT > 0) P.swapT -= dt;
  if (P.reload > 0) { P.reload -= dt; if (P.reload <= 0) { P.reload = 0; P.ammo[P.w] = magOf(P.w); } }
  if (firing) playerFire(); else semiLock = false;
  const hs = Math.hypot(P.vel.x, P.vel.z);
  if (P.onGround && P.slide <= 0) P.bob += dt * hs * 1.6;
  P.recoil = Math.max(0, P.recoil - dt * (P.w === 0 ? 12 : 6));
  const rl = P.reload > 0 ? Math.sin(Math.min(1, (P.reloadMax - P.reload) / P.reloadMax) * Math.PI) : 0;
  const sw = P.swapT > 0 ? Math.sin(P.swapT / .32 * Math.PI) : 0;
  const bobAmt = Math.min(hs/7, 1) * (1 - P.aimT * .8), a = P.aimT, W0 = WEAPONS[P.w];
  P.swayX *= Math.exp(-10*dt); P.swayY *= Math.exp(-10*dt);
  const swayK = 1 - a*.85;
  gun.position.set(
    GUN_BASE.x*(1 - a) + Math.cos(P.bob)*.01*bobAmt - P.slide*.05 - P.swayX*.25*swayK,
    GUN_BASE.y + (W0.aim[0] - GUN_BASE.y)*a + Math.abs(Math.sin(P.bob))*.014*bobAmt - rl*.12 - sw*.25 + P.swayY*.2*swayK,
    GUN_BASE.z + (W0.aim[1] - GUN_BASE.z)*a + P.recoil*(P.w === 1 ? .09 : .05)*(1 - a*.5));
  gun.rotation.set(P.recoil*.07*(1 - a*.6) - rl*.6 - sw*.5 + P.swayY*swayK, P.swayX*swayK + (1 - a)*.07, rl*.4 + (P.slide > 0 ? .25 : 0) + P.swayX*.6*swayK);
  // animaciones por arma
  const md = models[P.w][1];
  if (P.w === 0) md.mag.position.y = -rl*.12;
  if (P.w === 1) { const pk = P.fireCd > 0 ? Math.sin(Math.min(1, 1 - (P.fireCd - .2) / (W0.rate - .2)) * Math.PI) : 0; md.pump.position.z = Math.max(0, pk) * .07; }
  if (P.w === 2) {
    const charge = P.reload > 0 ? 0 : Math.min(1, 1 - Math.max(0, P.fireCd) / W0.rate);
    gm.cyan.color.setRGB(.37*charge + .05, .85*charge + .1, 1*charge + .15);
    md.coils.forEach((c, i) => c.scale.setScalar(1 + Math.sin(performance.now()/120 + i) * .04 * charge));
  }
  if (P.w === 2) models[2][0].visible = P.aimT < .8; // con el visor puesto, el arma se oculta
  if (P.w === 3) md.drum.rotation.z += (md.drumTarget - md.drum.rotation.z) * Math.min(1, dt * 14);
  if (P.fireCd < W0.rate - .04) { flash.visible = false; flashLight.intensity = 0; worldFlash.intensity = 0; }

  /* cámara */
  P.shake = Math.max(0, P.shake - dt * 2.2);
  const sh = P.shake * P.shake * .035;
  camera.position.set(P.pos.x, P.pos.y - P.crouchT * CROUCH_DROP, P.pos.z);
  camera.rotation.set(P.pitch + (Math.random()-.5)*sh, P.yaw + (Math.random()-.5)*sh, (P.slide > 0 ? -.06 : 0) * P.crouchT);

  /* regeneración y potenciadores */
  P.lastHurt += dt;
  if (P.lastHurt > 4 && P.hp < P.maxHp) P.hp = Math.min(P.maxHp, P.hp + 4 * P.regenMul * dt);
  if (P.overdrive > 0) P.overdrive -= dt;

  /* combo */
  if (comboT > 0) { comboT -= dt; if (comboT <= 0) combo = 0; }

  /* oleadas (o el modo ciudad) */
  if (isCity()) updateCity(dt, t);
  else if (spawnQueue.length) { spawnTimer -= dt; if (spawnTimer <= 0 && enemies.length < Math.min(12, 6 + Math.floor(wave/2))) { spawnEnemy(spawnQueue.shift()); spawnTimer = Math.max(.35, .9 - wave*.04); } }
  else if (!enemies.length) {
    if (intermission <= 0) {
      if (wave > 0) {
        score += wave*250; P.hp = Math.min(P.maxHp, P.hp + 20); P.nades = Math.min(P.maxNades, P.nades + 1);
        banner('DESPEJADO', `+${wave*250} bonus · +1 granada`);
        if (gameMode === 'campaign' && wave % WAVES_PER_LEVEL === 0) pendingLevel = true;
        showPerks();
      }
      intermission = wave > 0 ? 3.2 : 1.2;
    }
    intermission -= dt;
    if (intermission <= 0) { startWave(); intermission = 0; }
  }

  /* enemigos */
  const bodyY = P.pos.y - EYE + 1.1 - P.crouchT * .5;
  _body.set(P.pos.x, bodyY, P.pos.z);
  const kk = 1 - Math.exp(-3*dt);
  for (let ei = enemies.length - 1; ei >= 0; ei--) {
    const e = enemies[ei], p = e.g.position;
    if (e.soldier) { updateSoldier(e, dt, t); continue; }
    if (e.turret) { updateTurret(e, dt, t); continue; }
    if (e.surv) { updateSurv(e, dt, t); continue; }
    const dx = P.pos.x - p.x, dz = P.pos.z - p.z, dist = Math.hypot(dx, dz) || .001;
    const nx = dx/dist, nz = dz/dist;
    let mx, mz, ty;
    if (e.kami) {
      const wob = Math.sin(t*4 + e.phase) * .35;
      mx = nx - nz*wob; mz = nz + nx*wob;
      ty = dist > 6 ? 2.2 : bodyY;
    } else {
      if (e.flank) {
        // flanqueador: busca un punto al costado/espalda del jugador
        const fa = P.yaw + e.flank * 1.9, tx = P.pos.x - Math.sin(fa)*10, tz = P.pos.z - Math.cos(fa)*10;
        const fx = tx - p.x, fz = tz - p.z, fd = Math.hypot(fx, fz) || .001, fs = Math.min(1.25, fd * .25), wob = Math.sin(t*1.1 + e.phase) * .3;
        mx = fx/fd*fs - nz*wob; mz = fz/fd*fs + nx*wob;
      } else {
        const approach = Math.max(-1, Math.min(1, (dist - (e.boss ? 16 : e.heavy ? 8 : 11)) * .4));
        const side = Math.sin(t*.8 + e.phase) * (e.heavy ? .5 : .9);
        mx = nx*approach - nz*side; mz = nz*approach + nx*side;
      }
      ty = (e.boss ? 5 : e.heavy ? 3.2 : 2.6) + Math.sin(t*1.3 + e.phase)*.6;
    }
    for (let oi = 0; oi < enemies.length; oi++) { if (oi === ei) continue;
      const o = enemies[oi].g.position, ox = p.x - o.x, oz = p.z - o.z, od2 = ox*ox + oz*oz;
      if (od2 < 5.76 && od2 > 1e-4) { const od = Math.sqrt(od2); mx += ox/od * (2.4-od); mz += oz/od * (2.4-od); }
    }
    const ka = e.kami ? 1 - Math.exp(-5*dt) : kk;
    e.vel.x += (mx*e.speed - e.vel.x)*ka; e.vel.z += (mz*e.speed - e.vel.z)*ka; e.vel.y += ((ty - p.y)*(e.kami ? 4 : 2) - e.vel.y)*ka;
    p.addScaledVector(e.vel, dt);
    collideXZ(p, e.radius, p.y - e.radius, e.radius*2);
    p.x = Math.max(-LIM+.5, Math.min(LIM-.5, p.x)); p.z = Math.max(-LIM+.5, Math.min(LIM-.5, p.z));
    e.g.lookAt(P.pos.x, e.kami ? bodyY : P.pos.y, P.pos.z);
    if (e.ring2) { e.ring2.rotation.y += dt * 1.2; e.ring2.rotation.x = Math.cos(t*.7)*.5; }
    if (e.ring) { e.ring.rotation.z += dt * (e.heavy ? 1.5 : 3); e.ring.rotation.x = Math.sin(t*2 + e.phase)*.4; }
    else e.shell.rotation.z += dt * 8;
    if (e.flash > 0) { e.flash -= dt; e.shell.material = e.flash > 0 ? eMat.flash : e.base; }
    if (e.kami) {
      if (p.distanceTo(_body) < 1.4) {
        enemies.splice(ei, 1); scene.remove(e.g); targetsDirty = true;
        explode(p, 2.5, 2, 0, true); hurt(14, p);
        if (state !== 'playing') return;
      }
      continue;
    }
    e.fireCd -= dt;
    if (e.fireCd <= 0) {
      const d3 = _body.distanceTo(p);
      if (d3 < (e.boss ? 55 : 38) && hasLOS(p, _body)) {
        const lead = Math.random() < .4 ? d3 / 32 : 0;
        _aim.copy(_body).addScaledVector(P.vel, lead).sub(p).normalize();
        _aim.x += (Math.random()-.5)*.06; _aim.y += (Math.random()-.5)*.04; _aim.z += (Math.random()-.5)*.06; _aim.normalize();
        if (e.boss) {
          const n = 14;
          for (let q = 0; q < n; q++) { const ang = q/n*Math.PI*2 + t; enemyFire(e, _a.set(Math.cos(ang), -.12 + (Math.random()-.5)*.06, Math.sin(ang)).normalize()); }
          for (const ang of [-.07, 0, .07]) enemyFire(e, _a.copy(_aim).applyAxisAngle(UP, ang));
          sfx.boss(); e.fireCd = 3.4;
        } else {
          if (e.heavy) { for (const ang of [-.12, 0, .12]) enemyFire(e, _a.copy(_aim).applyAxisAngle(UP, ang)); }
          else enemyFire(e, _aim);
          sfx.enemy(d3);
          e.fireCd = (e.heavy ? 3 : 2.2 + Math.random()*1.6) / (1 + Math.min(wave, 15)*.03);
        }
      } else e.fireCd = .4;
    }
  }

  /* proyectiles */
  for (let i = shots.length - 1; i >= 0; i--) {
    const s = shots[i], sp = s.m.position; sp.addScaledVector(s.v, dt); s.life -= dt;
    if (s.life <= 0) { removeShot(i); continue; }
    if (sp.distanceToSquared(_body) < .45) { burst(sp, 0xff3d5a, 8, 4, .3); const from = s.from; removeShot(i); hurt(s.dmg, from); if (state !== 'playing') return; continue; }
    if (sp.y < 0 || boxAt(sp)) { burst(sp, 0xff8060, 8, 4, .3); removeShot(i); }
  }

  updateNades(dt);
  if (state !== 'playing') return;

  /* orbes */
  for (let i = orbs.length - 1; i >= 0; i--) {
    const o = orbs[i]; o.life -= dt;
    o.m.position.y = o.m.userData.baseY + Math.sin(t*3 + i)*.15; o.m.rotation.y += dt*2; o.m.rotation.x += dt;
    o.m.visible = o.life > 4 || Math.sin(t*20) > 0;
    const d = Math.hypot(o.m.position.x - P.pos.x, o.m.position.z - P.pos.z);
    const near = d < 1.4 && Math.abs(o.m.position.y - (P.pos.y - 1)) < 1.6;
    if (near && (o.type === 'od' || P.hp < P.maxHp)) {
      if (o.type === 'od') { P.overdrive = 8; banner('SOBRECARGA', 'Cadencia y daño aumentados por 8 s'); burst(o.m.position, 0xffb02e, 20, 5); }
      else { P.hp = Math.min(P.maxHp, P.hp + 25); burst(o.m.position, 0x5dffb0, 14, 4); }
      sfx.pickup(); scene.remove(o.m); orbs.splice(i, 1);
    }
    else if (o.life <= 0) { scene.remove(o.m); orbs.splice(i, 1); }
  }
}

function updateFx(dt) {
  lavaTex.offset.x += dt * .02; lavaTex.offset.y += dt * .012;
  updateParticles(dt);
  for (const tr of tracers) if (tr.life > 0) { tr.life -= dt; if (tr.life <= 0) tr.l.visible = false; }
  if (boomT > 0) { boomT -= dt; boomLight.intensity = Math.max(0, boomT / .25) * 6; }
}

