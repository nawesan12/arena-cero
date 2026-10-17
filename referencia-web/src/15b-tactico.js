/* ---------- enemigos tácticos: soldados, torretas y drones de vigilancia ---------- */
const cityState = { powered: true, antenna: true, intel: false, armory: false, lives: 3, time: 0, done: [], reinforceT: 20, stealthKills: 0, turretCols: [] };
const corpses = [];
const solMat = {
  armor: new THREE.MeshStandardMaterial({ color: 0x2b3238, metalness: .45, roughness: .55 }),
  armorC: new THREE.MeshStandardMaterial({ color: 0x3e2422, metalness: .5, roughness: .5 }),
  cloth: new THREE.MeshStandardMaterial({ color: 0x1f2427, roughness: .95 }),
  accent: new THREE.MeshStandardMaterial({ color: 0xd9792b, roughness: .6 }),
  gun: new THREE.MeshStandardMaterial({ color: 0x15181a, metalness: .6, roughness: .4 }),
  shield: new THREE.MeshStandardMaterial({ color: 0x8c9aa3, metalness: .8, roughness: .3, transparent: true, opacity: .9 }),
  turret: new THREE.MeshStandardMaterial({ color: 0x4b545b, metalness: .7, roughness: .4 }),
  flash: new THREE.MeshBasicMaterial({ color: 0xffffff })
};
const VISOR = { calm: 0x5fd8ff, search: 0xffb02e, combat: 0xff3d5a, off: 0x222a30 };
function bx(p, w, h, d, x, y, z, m) { const o = new THREE.Mesh(boxGeo, m); o.scale.set(w, h, d); o.position.set(x, y, z); o.castShadow = true; p.add(o); return o; }

function buildSoldierModel(kind) {
  const g = new THREE.Group(), r = {};
  const armor = kind === 'commander' ? solMat.armorC : solMat.armor;
  r.body = new THREE.Group(); g.add(r.body);
  r.legL = new THREE.Group(); r.legL.position.set(-.13, .95, 0); r.body.add(r.legL);
  r.legR = new THREE.Group(); r.legR.position.set(.13, .95, 0); r.body.add(r.legR);
  r.legMeshes = [bx(r.legL, .2, .92, .24, 0, -.46, 0, solMat.cloth), bx(r.legR, .2, .92, .24, 0, -.46, 0, solMat.cloth)];
  bx(r.legL, .22, .2, .3, 0, -.86, .04, armor); bx(r.legR, .22, .2, .3, 0, -.86, .04, armor);
  r.torso = bx(r.body, .54, .62, .32, 0, 1.3, 0, armor);
  bx(r.body, .4, .3, .06, 0, 1.36, .17, solMat.accent);
  bx(r.body, .42, .46, .2, 0, 1.32, -.24, solMat.cloth);
  r.headG = new THREE.Group(); r.headG.position.set(0, 1.66, 0); r.body.add(r.headG);
  r.head = bx(r.headG, .27, .29, .29, 0, .14, 0, solMat.cloth);
  r.helmet = bx(r.headG, .32, .15, .34, 0, .3, -.01, armor);
  r.visorMat = new THREE.MeshBasicMaterial({ color: VISOR.calm });
  r.visor = bx(r.headG, .25, .075, .03, 0, .16, .15, r.visorMat);
  bx(r.headG, .33, .04, .35, 0, .24, -.01, solMat.accent);
  for (const sx of [-1, 1]) { bx(r.body, .18, .12, .34, sx * .33, 1.56, 0, armor); bx(r.body, .19, .04, .35, sx * .33, 1.6, 0, solMat.accent); }
  bx(r.legL, .23, .14, .08, 0, -.5, .13, armor); bx(r.legR, .23, .14, .08, 0, -.5, .13, armor);
  bx(r.body, .56, .08, .34, 0, 1.0, 0, solMat.gun);
  r.arms = new THREE.Group(); r.arms.position.set(0, 1.45, .05); r.body.add(r.arms);
  bx(r.arms, .14, .14, .5, -.2, -.08, .2, armor); bx(r.arms, .14, .14, .5, .2, -.08, .2, armor);
  bx(r.arms, .08, .1, kind === 'sniper' ? 1.1 : .75, .05, -.04, kind === 'sniper' ? .6 : .45, solMat.gun);
  if (kind === 'sniper') bx(r.arms, .06, .06, .26, .05, .05, .3, solMat.gun);
  r.muzzle = new THREE.Object3D(); r.muzzle.position.set(.05, -.04, kind === 'sniper' ? 1.15 : .84); r.arms.add(r.muzzle);
  if (kind === 'commander') {
    r.shield = bx(r.body, 1.0, 1.35, .09, -.12, 1.0, .62, solMat.shield);
    bx(r.body, .14, .5, .1, -.12, 1.7, .62, solMat.accent);
    g.scale.setScalar(1.18);
  }
  return [g, r];
}

function spawnSoldier(op, spec) {
  const kind = spec.r === 'commander' ? 'commander' : spec.sniper ? 'sniper' : 'rifle';
  const [g, r] = buildSoldierModel(kind);
  const y = spec.y ?? .16;
  g.position.set(spec.x, y, spec.z);
  scene.add(g);
  const hp = kind === 'commander' ? 46 : kind === 'sniper' ? 4 : 6;
  const e = { g, m: r, soldier: true, kind, op, role: spec.r || 'guard', hp, maxHp: hp, vel: new V3(), radius: .5, color: 0xff3d5a,
    st: 'patrol', det: 0, lastSeen: new V3(), seeT: 99, visible: false, visT: Math.random() * .2, fireCd: 1 + Math.random(), burst: 0, burstT: 0,
    react: 0, yaw: Math.random() * Math.PI * 2, wp: Math.floor(Math.random() * Math.max(1, op.patrol.length)), wait: Math.random() * 2,
    anchor: new V3(spec.x, y, spec.z), leash: spec.leash ?? (spec.r === 'patrol' ? 22 : 12), target: null, cover: null, coverT: 0, crouch: 0, peekT: 0,
    nadeCd: 14 + Math.random() * 10, phase: Math.random() * 6, flank: Math.random() < .35, value: kind === 'commander' ? 1500 : kind === 'sniper' ? 200 : 120,
    aimT: 0, flash: 0, base: r.torso.material, tagged: false, floorY: y };
  e.shell = r.torso;
  e.hitMeshes = [r.torso, r.head, r.helmet, ...r.legMeshes];
  for (const m of e.hitMeshes) m.userData.enemy = e;
  r.head.userData.crit = r.helmet.userData.crit = true;
  if (r.shield) { r.shield.userData.shield = true; e.hitMeshes.push(r.shield); }
  if (kind === 'sniper') {
    const lg = new THREE.BufferGeometry(); lg.setAttribute('position', new THREE.BufferAttribute(new Float32Array(6), 3));
    e.laser = new THREE.Line(lg, new THREE.LineBasicMaterial({ color: 0xff2a3a, transparent: true, opacity: .8 }));
    e.laser.frustumCulled = false; e.laser.visible = false; scene.add(e.laser);
  }
  if (kind === 'commander') op.commander = e;
  enemies.push(e); targetsDirty = true;
  return e;
}

function spawnTurret(op, spec) {
  const g = new THREE.Group(), r = {};
  g.position.set(spec.x, .16, spec.z);
  bx(g, .9, .3, .9, 0, .15, 0, solMat.turret);
  const post = new THREE.Mesh(new THREE.CylinderGeometry(.22, .3, 1.3, 10), solMat.turret); post.position.y = .9; post.castShadow = true; g.add(post);
  r.head = new THREE.Group(); r.head.position.y = 1.7; g.add(r.head);
  r.headMesh = bx(r.head, .7, .45, .8, 0, 0, 0, solMat.turret);
  bx(r.head, .5, .1, .5, 0, .27, -.05, solMat.accent);
  for (const sx of [-.14, .14]) bx(r.head, .08, .08, .7, sx, -.04, .6, solMat.gun);
  r.eyeMat = new THREE.MeshBasicMaterial({ color: 0xff3d5a });
  bx(r.head, .16, .1, .04, 0, .06, .41, r.eyeMat);
  r.muzzle = new THREE.Object3D(); r.muzzle.position.set(0, -.04, .95); r.head.add(r.muzzle);
  const lg = new THREE.BufferGeometry(); lg.setAttribute('position', new THREE.BufferAttribute(new Float32Array(6), 3));
  r.laser = new THREE.Line(lg, new THREE.LineBasicMaterial({ color: 0xff3d5a, transparent: true, opacity: .55 }));
  r.laser.frustumCulled = false; scene.add(r.laser);
  scene.add(g);
  cityState.turretCols.push(addCollider(spec.x, spec.z, .9, 1.4, .9, .16));
  const e = { g, m: r, turret: true, op, hp: 12, maxHp: 12, vel: new V3(), radius: .6, color: 0xffb02e, baseYaw: spec.yaw, yaw: spec.yaw, pitch: 0,
    det: 0, st: 'patrol', lastSeen: new V3(), seeT: 99, visT: Math.random() * .2, fireCd: 0, burst: 0, burstT: 0, value: 250, flash: 0,
    base: solMat.turret, tagged: true, laserT: 0, laserLen: 40 };
  e.shell = r.headMesh;
  e.hitMeshes = [r.headMesh];
  r.headMesh.userData.enemy = e;
  enemies.push(e); targetsDirty = true;
  return e;
}

// drones de vigilancia: rutas por las avenidas con un cono de luz
const spotGeo = new THREE.ConeGeometry(6, 16, 20, 1, true); spotGeo.translate(0, -8, 0);
const spotMat = new THREE.MeshBasicMaterial({ color: 0xfff1b0, transparent: true, opacity: .12, blending: THREE.AdditiveBlending, depthWrite: false, side: THREE.DoubleSide });
function spawnSurvDrone(route, phase) {
  spawnEnemy('d');
  const e = enemies[enemies.length - 1];
  e.surv = { route, idx: phase % route.length, det: 0 };
  e.g.position.set(route[e.surv.idx][0], 16, route[e.surv.idx][1]);
  e.hp = e.maxHp = 5; e.value = 200;
  e.spot = new THREE.Mesh(spotGeo, spotMat); e.spot.position.set(0, 0, 0); scene.add(e.spot);
  return e;
}

function initCityEnemies() {
  for (const c of corpses) scene.remove(c.g); corpses.length = 0;
  for (const b of cityState.turretCols) removeCollider(b); cityState.turretCols = [];
  for (const op of OUTPOSTS) {
    op.alarm = 0; op.search = 0; op.done = false; op.commander = null; op.units = [];
    for (const s of op.soldiers) op.units.push(spawnSoldier(op, s));
    for (const s of op.snipers || []) op.units.push(spawnSoldier(op, Object.assign({ r: 'sniper', sniper: true, leash: 0 }, s)));
    for (const s of op.turrets) op.units.push(spawnTurret(op, s));
  }
  const routes = [
    [[-156, -156], [156, -156], [156, -66], [-156, -66]],
    [[-110, -156], [-110, 156], [-22, 156], [-22, -110]],
    [[156, 156], [22, 156], [22, 22], [156, 22]],
    [[66, -110], [66, 110], [110, 110], [110, -110]],
    [[-66, 66], [-156, 66], [-156, 110], [-66, 110]]
  ];
  routes.forEach((rt, i) => spawnSurvDrone(rt, i));
}

/* --- percepción --- */
function outpostOf(e) { return e.op; }
function raiseAlarm(op, knownPos) {
  if (!op || op.done && !op.units.some(u => u.hp > 0)) return;
  const fresh = op.alarm <= 0;
  op.alarm = 32;
  for (const u of op.units) {
    if (u.hp <= 0) continue;
    if (knownPos) { u.lastSeen.copy(knownPos); if (u.st !== 'combat') u.seeT = 1.5; }
    if (u.soldier) { u.st = 'combat'; if (u.react <= 0) u.react = .4 + Math.random() * .5; }
    else u.st = 'combat';
  }
  if (fresh) { sfx.siren(); banner(`ALERTA · ${op.full.toUpperCase()}`, cityState.antenna ? 'Pidieron refuerzos por radio' : 'La antena está caída: no llegan refuerzos'); }
}
function makeNoise(pos, radius) {
  if (!MAP || !MAP.city) return;
  for (const e of enemies) {
    if (!e.soldier || e.hp <= 0) continue;
    const d = e.g.position.distanceTo(pos);
    if (d > radius) continue;
    if (e.st === 'combat') { if (d < radius * .6) { e.lastSeen.copy(pos); } }
    else { e.st = 'search'; e.target = pos.clone().add(new V3((Math.random() - .5) * 6, 0, (Math.random() - .5) * 6)); e.det = Math.max(e.det, .5); e.searchT = 9; }
  }
}
const _eye = new V3(), _tp = new V3(), _mz = new V3();
function canSee(e, eyePos, range, cosFov, fwdX, fwdZ) {
  const dx = _body.x - eyePos.x, dz = _body.z - eyePos.z, dy = _body.y - eyePos.y, d = Math.hypot(dx, dy, dz);
  if (d > range) return 0;
  if (d > 2.5 && cosFov > -1) { const h = Math.hypot(dx, dz) || 1; if ((dx * fwdX + dz * fwdZ) / h < cosFov) return 0; }
  if (!hasLOS(eyePos, _body)) return 0;
  return d || .01;
}
function stanceFactor() {
  const hs = Math.hypot(P.vel.x, P.vel.z);
  return (P.crouchT > .5 ? .5 : 1) * (hs > 8.5 ? 1.45 : hs < .5 ? .75 : 1);
}

/* --- disparos enemigos (hitscan con trazadora) --- */
function enemyShot(e, from, dmg, acc) {
  const d = from.distanceTo(_body);
  const hs = Math.hypot(P.vel.x, P.vel.z);
  let p = acc - d * .009 - (hs > 8 ? .2 : hs > 3 ? .12 : 0) - (P.hook || P.dashT > 0 ? .35 : 0) - (P.crouchT > .5 ? .08 : 0);
  p = Math.max(.04, Math.min(.6, p));
  _tp.copy(_body);
  if (Math.random() < p) hurt(dmg, from);
  else { _tp.x += (Math.random() - .5) * 2.2; _tp.y += (Math.random() - .3) * 1.4; _tp.z += (Math.random() - .5) * 2.2; if (d < 30) sfx.whiz(); }
  tracer(from, _tp, 0xffc66a, .05);
  sfx.eshot(d, e.kind === 'sniper');
  burst(from, 0xffd27a, 2, 2, .08);
}

/* --- soldado --- */
function soldierMove(e, tx, tz, speed, dt) {
  const p = e.g.position, dx = tx - p.x, dz = tz - p.z, d = Math.hypot(dx, dz);
  if (d < .3) return true;
  const s = Math.min(speed * dt, d);
  p.x += dx / d * s; p.z += dz / d * s;
  collideXZ(p, .38, p.y, 1.8);
  // correa: no se alejan de su puesto
  const ax = p.x - e.anchor.x, az = p.z - e.anchor.z, ad = Math.hypot(ax, az), lim = Math.max(e.leash, 1);
  if (e.leash > 0 && ad > lim) { p.x = e.anchor.x + ax / ad * lim; p.z = e.anchor.z + az / ad * lim; }
  if (e.leash <= 0) { p.x = e.anchor.x; p.z = e.anchor.z; }
  const gy = groundAt(p.x, p.z, p.y + .3); p.y = gy;
  e.moving = true; e.moveYaw = Math.atan2(dx, dz);
  return false;
}
function pickCover(e) {
  const op = e.op, p = e.g.position;
  let best = null, bs = 1e9;
  for (const c of op.cover) {
    const dSelf = Math.hypot(c[0] - p.x, c[1] - p.z);
    if (dSelf > 16) continue;
    const dPl = Math.hypot(c[0] - P.pos.x, c[1] - P.pos.z);
    if (dPl < 6 || dPl > 40) continue;
    _eye.set(c[0], e.floorY + .7, c[1]);
    if (hasLOS(_eye, _body)) continue;            // agachado no lo ven
    _eye.y = e.floorY + 1.6;
    const peek = hasLOS(_eye, _body);             // parado puede disparar
    const sc = dSelf + (peek ? 0 : 8) + Math.random() * 2;
    if (sc < bs) { bs = sc; best = c; }
  }
  return best;
}
function updateSoldier(e, dt, t) {
  const p = e.g.position, m = e.m;
  e.moving = false;
  if (e.flash > 0) { e.flash -= dt; m.torso.material = e.flash > 0 ? solMat.flash : e.base; }
  // percepción (cada ~0.15 s)
  e.visT -= dt;
  const kindRange = e.kind === 'sniper' ? 95 : 42;
  if (e.visT <= 0) {
    e.visT = .15;
    _eye.set(p.x, p.y + (e.crouch > .5 ? 1.1 : 1.72) * e.g.scale.y, p.z);
    const fx = Math.sin(e.yaw), fz = Math.cos(e.yaw);
    const combat = e.st === 'combat';
    const d = canSee(e, _eye, combat ? kindRange * 1.25 : kindRange, combat ? .05 : .42, fx, fz);
    const was = e.visible;
    e.visible = d > 0;
    // cada vez que vuelve a verte tarda en reaccionar (asomarse no es un suicidio)
    if (e.visible && !was && combat) e.react = Math.max(e.react, .8 + Math.random() * .8 + d * .012);
    if (e.visible) {
      e.lastSeen.copy(_body); e.seeT = 0;
      if (!combat) {
        const range = kindRange;
        const rate = (d < 4 ? 3.5 : .8 * (1 - d / range) + .12) * stanceFactor();
        e.det = Math.min(1, e.det + rate * .15);
        if (e.det >= 1) { e.st = 'combat'; e.react = .45 + Math.random() * .4; raiseAlarm(e.op, _body); }
        else if (e.det > .45 && e.st === 'patrol') { e.st = 'search'; e.target = _body.clone(); e.searchT = 8; }
      }
    } else {
      if (e.st !== 'combat') e.det = Math.max(0, e.det - .04);
      // cadáveres a la vista
      if (e.st === 'patrol') for (const c of corpses) {
        if (c.seen || c.g.position.distanceTo(p) > 20) continue;
        const cx = c.g.position.x - p.x, cz = c.g.position.z - p.z, h = Math.hypot(cx, cz) || 1;
        if ((cx * fx + cz * fz) / h < .3) continue;
        _tp.set(c.g.position.x, c.g.position.y + .4, c.g.position.z);
        if (hasLOS(_eye, _tp)) { c.seen = true; e.st = 'search'; e.target = c.g.position.clone(); e.searchT = 10; e.corpse = c; e.det = Math.max(e.det, .6); break; }
      }
    }
  }
  if (e.st === 'combat') e.seeT += dt;
  // máquina de estados
  let faceYaw = null;
  if (e.st === 'patrol') {
    if (e.role === 'patrol' && e.op.patrol.length) {
      const w = e.op.patrol[e.wp % e.op.patrol.length];
      if (e.wait > 0) { e.wait -= dt; faceYaw = e.yaw + Math.sin(t * .8 + e.phase) * .02; }
      else if (soldierMove(e, w[0], w[1], 1.9, dt)) { e.wp++; e.wait = 1.5 + Math.random() * 2.5; }
    } else {
      faceYaw = e.anchorYaw ?? (e.anchorYaw = e.yaw);
      faceYaw = e.anchorYaw + Math.sin(t * .35 + e.phase) * .9;
    }
  } else if (e.st === 'search') {
    e.searchT -= dt;
    if (e.target && soldierMove(e, e.target.x, e.target.z, 3.2, dt)) {
      faceYaw = e.yaw + dt * 1.4;
      if (e.corpse && !e.corpse.reported) { e.corpse.reported = true; raiseAlarm(e.op, null); }
    }
    if (e.searchT <= 0) { e.st = 'patrol'; e.target = null; e.corpse = null; }
  } else if (e.st === 'combat') {
    e.react -= dt; e.nadeCd -= dt; e.coverT -= dt;
    const lp = e.lastSeen;
    faceYaw = Math.atan2(lp.x - p.x, lp.z - p.z);
    if (e.kind === 'sniper') {
      if (e.visible && e.react <= 0) e.aimT += dt; else e.aimT = Math.max(0, e.aimT - dt * 2);
      if (e.aimT >= 2.6) { e.aimT = 0; e.react = 3; m.muzzle.getWorldPosition(_mz); enemyShot(e, _mz, 24, .75); }
    } else {
      // cobertura: buscar, ir, asomarse y disparar
      if (e.coverT <= 0 && e.leash > 0) { e.cover = pickCover(e); e.coverT = 6 + Math.random() * 4; }
      let atCover = true;
      if (e.cover) atCover = soldierMove(e, e.cover[0], e.cover[1], 4.2, dt);
      else if (!e.visible && e.seeT > 2.5) {
        // avanzar hacia la última posición conocida (los flanqueadores abren por un costado)
        let tx = lp.x, tz = lp.z;
        if (e.flank) { const a = Math.atan2(lp.x - p.x, lp.z - p.z) + Math.PI / 2; tx += Math.sin(a) * 7; tz += Math.cos(a) * 7; }
        atCover = soldierMove(e, tx, tz, 3.6, dt);
      }
      if (atCover && e.cover) {
        e.peekT -= dt;
        if (e.peekT <= 0) { e.peekT = e.crouch > .5 ? 1.6 + Math.random() : 1.4 + Math.random() * 1.2; e.wantCrouch = !(e.crouch > .5); }
      } else e.wantCrouch = false;
      // granadas cuando el jugador se esconde
      if (!e.visible && e.seeT > 2.5 && e.seeT < 9 && e.nadeCd <= 0) {
        const dd = lp.distanceTo(p);
        if (dd > 6 && dd < 26 && Math.random() < .5) { enemyNade(e, lp); } e.nadeCd = 16 + Math.random() * 10;
      }
      // ráfagas
      e.fireCd -= dt;
      const canShoot = e.visible && e.react <= 0 && e.crouch < .5;
      // como mucho dos tiradores por puesto al mismo tiempo
      const shooters = e.op.units.reduce((n, u) => n + (u !== e && u.soldier && u.hp > 0 && u.burst > 0 ? 1 : 0), 0);
      if (e.burst > 0) {
        e.burstT -= dt;
        if (e.burstT <= 0) { e.burst--; e.burstT = .11; m.muzzle.getWorldPosition(_mz); if (canShoot) enemyShot(e, _mz, e.kind === 'commander' ? 5 : 4, e.kind === 'commander' ? .48 : .4); }
      } else if (canShoot && e.fireCd <= 0 && shooters < 2) { e.burst = e.kind === 'commander' ? 4 : 3; e.burstT = .25; e.fireCd = 1.8 + Math.random() * 1.6; }
      if (e.seeT > 25 && e.op.alarm <= 0) { e.st = 'search'; e.target = lp.clone(); e.searchT = 10; }
    }
  }
  // agacharse, girar y animar
  e.crouch += ((e.wantCrouch && e.st === 'combat' ? 1 : 0) - e.crouch) * Math.min(1, dt * 8);
  if (faceYaw === null && e.moving) faceYaw = e.moveYaw;
  if (faceYaw !== null) { let dy = faceYaw - e.yaw; dy = Math.atan2(Math.sin(dy), Math.cos(dy)); e.yaw += dy * Math.min(1, dt * (e.st === 'combat' ? 9 : 4)); }
  e.g.rotation.y = e.yaw;
  const sw = e.moving ? Math.sin(t * 9 + e.phase) * .55 : 0;
  m.legL.rotation.x = sw - e.crouch * 1.2; m.legR.rotation.x = -sw - e.crouch * .2;
  m.body.position.y = -e.crouch * .48;
  const aimPitch = e.st === 'combat' ? Math.atan2(e.lastSeen.y - (p.y + 1.45), Math.hypot(e.lastSeen.x - p.x, e.lastSeen.z - p.z)) : -.35;
  m.arms.rotation.x = -aimPitch;
  m.visorMat.color.setHex(e.st === 'combat' ? VISOR.combat : e.st === 'search' ? VISOR.search : VISOR.calm);
  // láser del francotirador
  if (e.laser) {
    const on = e.st === 'combat' && e.aimT > .05;
    e.laser.visible = on;
    if (on) { m.muzzle.getWorldPosition(_mz); const a = e.laser.geometry.attributes.position; a.setXYZ(0, _mz.x, _mz.y, _mz.z); a.setXYZ(1, _body.x, _body.y + .2, _body.z); a.needsUpdate = true; e.laser.material.opacity = .3 + e.aimT / 2.6 * .7; }
  }
}
// granada enemiga: parábola hacia la última posición conocida
function enemyNade(e, target) {
  const p = e.g.position;
  const m = new THREE.Mesh(nadeGeo, nadeMat); m.add(new THREE.Mesh(nadeLightGeo, new THREE.MeshBasicMaterial({ color: 0xff3d5a })));
  m.position.set(p.x, p.y + 1.7, p.z); scene.add(m);
  const dx = target.x - p.x, dz = target.z - p.z, d = Math.hypot(dx, dz), T = Math.max(.8, d / 13);
  const v = new V3(dx / T, (target.y - m.position.y + .5 * 22 * T * T) / T, dz / T);
  nades.push({ m, v, fuse: T + .9, beep: .2, enemy: true });
}

/* --- torreta --- */
function updateTurret(e, dt, t) {
  const r = e.m, p = e.g.position;
  if (e.flash > 0) { e.flash -= dt; r.headMesh.material = e.flash > 0 ? solMat.flash : e.base; }
  const powered = cityState.powered;
  r.laser.visible = powered;
  r.eyeMat.color.setHex(powered ? (e.st === 'combat' ? 0xff3d5a : 0xffb02e) : 0x1a1d20);
  if (!powered) { r.head.rotation.x += (.5 - r.head.rotation.x) * Math.min(1, dt * 2); return; }
  e.visT -= dt;
  _eye.set(p.x, p.y + 1.7, p.z);
  if (e.visT <= 0) {
    e.visT = .12;
    const fx = Math.sin(e.yaw), fz = Math.cos(e.yaw);
    const d = canSee(e, _eye, e.st === 'combat' ? 50 : 40, e.st === 'combat' ? .5 : .86, fx, fz);
    const was = e.visible; e.visible = d > 0;
    if (e.visible && !was) e.react = .9 + Math.random() * .5;
    if (e.visible) {
      e.lastSeen.copy(_body); e.seeT = 0;
      if (e.st !== 'combat') { e.det += (1.4 * (1 - d / 40) + .3) * stanceFactor() * .12; if (e.det >= 1) { e.st = 'combat'; raiseAlarm(e.op, _body); } }
    } else { e.det = Math.max(0, e.det - .03); }
  }
  e.seeT += dt;
  let ty;
  if (e.st === 'combat' && e.seeT < 4) {
    ty = Math.atan2(e.lastSeen.x - p.x, e.lastSeen.z - p.z);
    const pitch = -Math.atan2(e.lastSeen.y - (p.y + 1.7), Math.hypot(e.lastSeen.x - p.x, e.lastSeen.z - p.z));
    r.head.rotation.x += (pitch - r.head.rotation.x) * Math.min(1, dt * 6);
    e.fireCd -= dt; e.react = (e.react || 0) - dt;
    if (e.burst > 0) { e.burstT -= dt; if (e.burstT <= 0) { e.burst--; e.burstT = .1; r.muzzle.getWorldPosition(_mz); if (e.visible) enemyShot(e, _mz, 3, .3); } }
    else if (e.visible && e.fireCd <= 0 && e.react <= 0) { e.burst = 3; e.fireCd = 2.2; }
  } else {
    if (e.st === 'combat' && e.seeT > 10) e.st = 'patrol';
    ty = e.baseYaw + Math.sin(t * .45 + e.baseYaw) * .95;
    r.head.rotation.x += (0 - r.head.rotation.x) * Math.min(1, dt * 3);
  }
  let dy = ty - e.yaw; dy = Math.atan2(Math.sin(dy), Math.cos(dy)); e.yaw += dy * Math.min(1, dt * (e.st === 'combat' ? 5 : 2));
  r.head.rotation.y = e.yaw;
  // láser: hasta la primera superficie
  e.laserT -= dt;
  r.muzzle.getWorldPosition(_mz);
  _d.set(Math.sin(e.yaw) * Math.cos(r.head.rotation.x), -Math.sin(r.head.rotation.x), Math.cos(e.yaw) * Math.cos(r.head.rotation.x));
  if (e.laserT <= 0) { e.laserT = .1; e.laserLen = rayWorld(_mz, _d, 45); }
  const a = r.laser.geometry.attributes.position;
  a.setXYZ(0, _mz.x, _mz.y, _mz.z); a.setXYZ(1, _mz.x + _d.x * e.laserLen, _mz.y + _d.y * e.laserLen, _mz.z + _d.z * e.laserLen); a.needsUpdate = true;
}

/* --- dron de vigilancia --- */
function updateSurv(e, dt, t) {
  const s = e.surv, p = e.g.position, w = s.route[s.idx];
  const dx = w[0] - p.x, dz = w[1] - p.z, d = Math.hypot(dx, dz);
  if (d < 1.5) s.idx = (s.idx + 1) % s.route.length;
  else { p.x += dx / d * 6.5 * dt; p.z += dz / d * 6.5 * dt; }
  p.y += (16 + Math.sin(t + e.phase) * .6 - p.y) * Math.min(1, dt * 2);
  const yaw = Math.atan2(dx, dz);
  e.g.rotation.set(0, yaw, 0);
  if (e.ring) e.ring.rotation.z += dt * 2;
  if (e.flash > 0) { e.flash -= dt; e.shell.material = e.flash > 0 ? eMat.flash : e.base; }
  const sx = p.x + Math.sin(yaw) * 5, sz = p.z + Math.cos(yaw) * 5;
  e.spot.position.set(p.x, p.y, p.z); e.spot.lookAt(sx, 0, sz); e.spot.rotateX(-Math.PI / 2);
  const hd = Math.hypot(P.pos.x - sx, P.pos.z - sz);
  if (hd < 6.5 && P.pos.y < 9 && hasLOS(p, _body)) {
    s.det += dt * 1.6 * stanceFactor();
    if (s.det >= 1) {
      let best = null, bd = 1e9; for (const op of OUTPOSTS) { const dd = Math.hypot(op.cx - P.pos.x, op.cz - P.pos.z); if (!op.done && dd < bd) { bd = dd; best = op; } }
      raiseAlarm(best, _body); e.surv = null; e.hunting = true; scene.remove(e.spot); e.spot = null; sfx.alertBeep();
    }
  } else s.det = Math.max(0, s.det - dt * .5);
}

/* --- refuerzos --- */
function updateReinforcements(dt) {
  const hot = OUTPOSTS.find(o => o.alarm > 0);
  if (!hot || !cityState.antenna || cityState.drones === false) return;
  cityState.reinforceT -= dt;
  if (cityState.reinforceT > 0) return;
  cityState.reinforceT = 28;
  const hunters = enemies.filter(e => e.hunting).length;
  for (let i = 0; i < 2 && hunters + i < 4; i++) {
    spawnEnemy('d'); const e = enemies[enemies.length - 1];
    e.hunting = true; e.g.position.set(hot.cx + (Math.random() - .5) * 30, 26, hot.cz + (Math.random() - .5) * 30);
  }
  banner('REFUERZOS', 'Drones en camino. Destruí la antena para cortarlos.');
}

/* --- cadáveres --- */
function killSoldier(e) {
  const i = enemies.indexOf(e); if (i >= 0) enemies.splice(i, 1);
  targetsDirty = true;
  for (const mm of e.hitMeshes) mm.userData.enemy = null;
  if (e.laser) { scene.remove(e.laser); }
  e.m.visorMat.color.setHex(VISOR.off);
  corpses.push({ g: e.g, t: 0, seen: false, fall: Math.random() < .5 ? 1 : -1 });
  if (corpses.length > 30) { const c = corpses.shift(); scene.remove(c.g); }
}
function updateCorpses(dt) {
  for (const c of corpses) {
    if (c.t < .5) { c.t += dt; const k = Math.min(1, c.t / .5); c.g.rotation.x = -k * Math.PI / 2 * c.fall * .98; c.g.position.y = Math.max(c.g.position.y - dt * .6, groundAt(c.g.position.x, c.g.position.z, c.g.position.y + .2) + .15 * k); }
  }
}
function killTurret(e) {
  const i = enemies.indexOf(e); if (i >= 0) enemies.splice(i, 1);
  targetsDirty = true;
  scene.remove(e.m.laser);
  e.m.head.visible = false;
  burst(e.g.position.clone().setY(1.9), 0xffb02e, 30, 8, .7); burst(e.g.position.clone().setY(1.9), 0x2a2a2a, 14, 4, 1.2);
  sfx.boom(); makeNoise(e.g.position, 60);
}
