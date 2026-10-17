/* ---------- modo ciudad: objetivos, interacción, marcadores, mapa y vidas ---------- */
const OBJ_TIME = { antena: 2.5, sub: 3, datos: 4, arsenal: 3, obra: 3.5, trenes: 2.5 };
let interact = null, holdT = 0, fWas = false, charge = null, supplyCd = 0, mapOpen = false, tagT = 0;
function isCity() { return !!(MAP && MAP.city); }
function initCity() {
  Object.assign(cityState, { powered: true, antenna: true, drones: true, intel: false, armory: false, lives: 3, time: 0, done: [], reinforceT: 20, stealthKills: 0, turretCols: [] });
  charge = null; holdT = 0; supplyCd = 0;
  for (const op of OUTPOSTS) { op.everAlarm = false; op.cleared = false; }
  SUPPLY.length = 0;
  SUPPLY.push({ pos: SAFE_POS.clone(), name: 'Refugio', active: true, home: true });
  P.unlocked = [true, true, false, false];
  P.nades = 2;
  initCityEnemies();
  renderObjList();
}
function unitsAlive(op) { return op.units.filter(u => u.hp > 0 && enemies.includes(u)); }

/* --- muerte de soldados y torretas --- */
function cityKill(e) {
  const op = e.op, stealth = op && op.alarm <= 0;
  if (e.soldier) killSoldier(e); else killTurret(e);
  kills++;
  const v = e.value + (stealth && e.soldier ? 60 : 0);
  score += v; if (stealth && e.soldier) cityState.stealthKills++;
  if (P.leech) P.hp = Math.min(P.maxHp, P.hp + P.leech);
  if (e.kind === 'commander') completeObjective(op);
  if (op && !op.cleared && unitsAlive(op).filter(u => u.soldier).length === 0) {
    op.cleared = true;
    SUPPLY.push({ pos: new V3(op.cx, 1, op.cz), name: op.name, active: true });
    banner('PUESTO DESPEJADO', `${op.full}: ahora es punto de reabastecimiento`);
  }
}
function completeObjective(op) {
  if (op.done) return;
  op.done = true; cityState.done.push(op.id);
  const ghost = !op.everAlarm;
  score += 1500 + (ghost ? 1000 : 0);
  sfx.objective();
  if (op.id === 'sub') { cityState.powered = false; burst(op.objPos, 0x5fd8ff, 40, 7, .8); }
  if (op.id === 'datos') cityState.intel = true;
  if (op.id === 'obra') { cityState.lives = Math.min(4, cityState.lives + 1); for (const e of enemies) if (e.turret || e.kind === 'sniper') e.tagged = true; }
  if (op.id === 'trenes') {
    cityState.drones = false;
    for (let i = enemies.length - 1; i >= 0; i--) { const e = enemies[i]; if (e.surv || e.hunting) { burst(e.g.position, 0xff6a2e, 24, 7, .7); e.hp = 0; killEnemy(e); } }
  }
  if (op.id === 'arsenal') { cityState.armory = true; P.unlocked = [true, true, true, true]; P.ammo = WEAPONS.map((_, i) => magOf(i)); P.nades = P.maxNades; }
  banner(op.full.toUpperCase(), op.effect + (ghost ? ' · +1000 sin ser visto' : ''));
  renderObjList();
  if (cityState.done.length >= OUTPOSTS.length) { setTimeout(victory, 1500); return; }
  setTimeout(() => { if (state === 'playing') showPerks(); }, 1600);
}
function victory() {
  if (state !== 'playing' && state !== 'perk') return;
  const tb = Math.max(0, Math.round(8000 - cityState.time * 4));
  score += 5000 + tb + cityState.lives * 1000;
  state = 'over'; firing = aiming = false;
  if (document.pointerLockElement) document.exitPointerLock();
  releaseHook(false);
  const isBest = score > best;
  if (isBest) { best = score; try { localStorage.setItem('arena-cero-best-' + lbKey(), best); } catch (e) {} }
  showOverlay('win', isBest);
  submitScore();
}
function cityRespawn() {
  cityState.lives--;
  let sp = SUPPLY[0].pos, bd = 1e9;
  for (const s of SUPPLY) { const d = s.pos.distanceTo(P.pos); if (d < bd) { bd = d; sp = s.pos; } }
  P.hp = P.maxHp; P.vel.set(0, 0, 0); P.pos.set(sp.x + 1.5, sp.y + EYE, sp.z + 1.5); P.lastHurt = 10;
  releaseHook(false);
  while (shots.length) removeShot(shots.length - 1);
  for (let i = nades.length - 1; i >= 0; i--) if (nades[i].enemy) { scene.remove(nades[i].m); nades.splice(i, 1); }
  for (const op of OUTPOSTS) op.alarm = 0;
  for (let i = enemies.length - 1; i >= 0; i--) {
    const e = enemies[i];
    if (e.hunting) { scene.remove(e.g); enemies.splice(i, 1); continue; }
    if (e.soldier) { e.st = 'patrol'; e.det = 0; e.seeT = 99; e.cover = null; e.g.position.copy(e.anchor); }
    if (e.turret) { e.st = 'patrol'; e.det = 0; }
  }
  targetsDirty = true;
  hurtFlash = 1;
  banner('CAÍSTE', cityState.lives > 1 ? `Te quedan ${cityState.lives} intentos` : 'Último intento');
}

/* --- interacción (F) --- */
function findInteract() {
  // derribo sigiloso: por la espalda, sin que te haya visto
  for (const e of enemies) {
    if (!e.soldier || e.st === 'combat' || e.kind === 'commander') continue;
    const p = e.g.position, d = Math.hypot(p.x - P.pos.x, p.z - P.pos.z);
    if (d > 2.3 || Math.abs(p.y - (P.pos.y - EYE)) > 1.2) continue;
    const fx = Math.sin(e.yaw), fz = Math.cos(e.yaw), tx = (P.pos.x - p.x) / d, tz = (P.pos.z - p.z) / d;
    if (fx * tx + fz * tz < -.15) return { kind: 'takedown', e, label: 'F · Neutralizar en silencio' };
  }
  const feet = P.pos.y - EYE;
  for (const op of OUTPOSTS) {
    if (op.done || !op.objPos || (charge && charge.op === op)) continue;
    const d = Math.hypot(op.objPos.x - P.pos.x, op.objPos.z - P.pos.z);
    if (d < 2.6 && Math.abs(op.objPos.y - 1 - feet) < 1.4) return { kind: 'obj', op, label: `Mantené F · ${op.task}`, time: OBJ_TIME[op.id] || 3 };
  }
  for (const s of SUPPLY) {
    if (!s.active) continue;
    if (Math.hypot(s.pos.x - P.pos.x, s.pos.z - P.pos.z) < 2.4 && Math.abs(s.pos.y - 1 - feet) < 1.4)
      return { kind: 'supply', s, label: supplyCd > 0 ? `Reabastecimiento listo en ${Math.ceil(supplyCd)} s` : 'F · Reabastecerse', ready: supplyCd <= 0 };
  }
  return null;
}
function updateInteract(dt) {
  supplyCd -= dt;
  interact = findInteract();
  const f = !!keys.KeyF, pressed = f && !fWas; fWas = f;
  if (interact && interact.kind === 'obj' && f) {
    holdT += dt;
    if (holdT >= interact.time) {
      holdT = 0; const op = interact.op;
      if (op.id === 'antena' || op.id === 'trenes') { charge = { op, t: 4, beep: 0 }; sfx.plant(); banner('CARGA PLANTADA', 'Alejate: explota en 4 segundos'); makeNoise(op.objPos, 18); }
      else completeObjective(op);
    }
  } else holdT = 0;
  if (interact && pressed) {
    if (interact.kind === 'takedown') {
      const e = interact.e; sfx.takedown();
      burst(e.g.position.clone().setY(e.g.position.y + 1.3), 0xfff0e0, 6, 3, .3);
      queueNum(e, e.hp, true); e.hp = 0; cityKill(e); hitMark('kill');
    } else if (interact.kind === 'supply' && interact.ready) {
      P.hp = P.maxHp; P.nades = P.maxNades; P.ammo = WEAPONS.map((_, i) => magOf(i)); supplyCd = 20; sfx.pickup();
      banner('REABASTECIDO', 'Integridad, munición y granadas al máximo');
    }
  }
  // carga explosiva en la antena
  if (charge) {
    charge.t -= dt; charge.beep -= dt;
    if (charge.beep <= 0) { sfx.beep(); charge.beep = charge.t < 1.5 ? .12 : .4; }
    if (charge.t <= 0) {
      const op = charge.op; charge = null;
      explode(op.objPos, 7, 10, 40, true); makeNoise(op.objPos, 90);
      if (op.id === 'antena') {
        for (let h = 1; h < 24; h += 4) { _c.set(op.cx, h, op.cz); burst(_c, 0xffb02e, 18, 8, .8); }
        cityState.antenna = false; op.toppling = 0;
        for (const b of op.antennaCols) removeCollider(b);
      } else {
        for (let i = 0; i < 6; i++) { _c.set(op.cx + 1 + Math.random() * 10, 2 + Math.random() * 7, op.cz + 7 + Math.random() * 5); burst(_c, i % 2 ? 0xff6a2e : 0x2a2a2a, 30, 9, 1.4); }
        P.shake = Math.min(1, P.shake + .6);
      }
      raiseAlarm(op, null);
      completeObjective(op);
    }
  }
}
function updateAntennaFall(dt) {
  const op = OUTPOSTS.find(o => o.id === 'antena');
  if (!op || op.toppling === undefined || !op.antennaGroup) return;
  op.toppling += dt;
  const k = Math.min(1, op.toppling / 2.4);
  op.antennaGroup.rotation.z = -k * k * Math.PI / 2 * .96;
  if (k >= 1 && !op.fell) { op.fell = true; _c.set(op.cx + 20, 1, op.cz); burst(_c, 0x6a6a6a, 50, 9, 1.4); sfx.boom(); P.shake = Math.min(1, P.shake + .5); }
}

/* --- alarma por puesto --- */
function updateOutposts(dt) {
  for (const op of OUTPOSTS) {
    if (op.alarm > 0) {
      op.everAlarm = true;
      const seen = op.units.some(u => u.hp > 0 && u.visible && u.st === 'combat');
      if (!seen) op.alarm -= dt;
      if (op.alarm <= 0) {
        for (const u of op.units) if (u.hp > 0 && u.soldier && u.st === 'combat') { u.st = 'search'; u.target = u.lastSeen.clone(); u.searchT = 14; u.det = .5; u.cover = null; }
        banner('BÚSQUEDA', `${op.full}: te perdieron el rastro`);
      }
    }
  }
  updateReinforcements(dt);
}
function cityAlert() {
  if (OUTPOSTS.some(o => o.alarm > 0)) return 2;
  if (enemies.some(e => e.soldier && e.st === 'search')) return 1;
  return 0;
}

/* --- marcado de enemigos (T o rueda del mouse) --- */
function tryTag() {
  if (!isCity()) return;
  camera.getWorldDirection(_d);
  let best = null, bs = .06;
  for (const e of enemies) {
    if (e.tagged || e.hp <= 0) continue;
    _tp.copy(e.g.position); _tp.y += e.soldier ? 1.2 : e.turret ? 1.7 : 0;
    _c.copy(_tp).sub(camera.position); const d = _c.length(); if (d > 140) continue; _c.divideScalar(d);
    const ang = Math.acos(Math.min(1, _c.dot(_d)));
    if (ang < bs && hasLOS(camera.position, _tp)) { bs = ang; best = e; }
  }
  if (best) { best.tagged = true; sfx.tag(); }
}

/* --- marcadores en pantalla --- */
const mkEl = $('markers'), mkPool = [];
for (let i = 0; i < 64; i++) { const d = document.createElement('div'); d.className = 'mk'; d.hidden = true; d.innerHTML = '<b></b><span></span>'; mkEl.appendChild(d); mkPool.push(d); }
let mkUsed = 0;
const _vc = new V3();
function mk(x, y, z, cls, icon, label, clamp, fill) {
  if (mkUsed >= mkPool.length) return;
  _v.set(x, y, z).project(camera);
  const behind = _v.z > 1;
  let sx = (_v.x * .5 + .5) * innerWidth, sy = (-_v.y * .5 + .5) * innerHeight;
  const off = behind || sx < 0 || sx > innerWidth || sy < 0 || sy > innerHeight;
  if (off && !clamp) return;
  if (off) {
    // fuera de cuadro: solo el ícono, sobre una elipse alrededor de la mira según el rumbo
    _vc.set(x, y, z).applyMatrix4(camera.matrixWorldInverse);
    const th = Math.atan2(_vc.x, -_vc.z);
    sx = innerWidth / 2 + Math.sin(th) * innerWidth * .4; sy = innerHeight / 2 - Math.cos(th) * innerHeight * .3;
    label = '';
  }
  const el = mkPool[mkUsed++];
  el.hidden = false;
  if (el.dataset.c !== cls) { el.dataset.c = cls; el.className = 'mk ' + cls; }
  const b = el.firstChild, s = el.lastChild;
  if (b.textContent !== icon) b.textContent = icon;
  if (s.textContent !== label) s.textContent = label;
  if (fill !== undefined) el.style.setProperty('--f', fill.toFixed(2));
  el.style.transform = `translate(${sx.toFixed(0)}px,${sy.toFixed(0)}px)`;
}
function updateMarkers() {
  mkUsed = 0;
  if (state === 'playing' && isCity()) {
    for (const op of OUTPOSTS) {
      if (op.done) continue;
      const pos = op.objPos || (op.commander && op.commander.hp > 0 ? op.commander.g.position : null) || new V3(op.cx, 0, op.cz);
      const d = Math.hypot(pos.x - P.pos.x, pos.z - P.pos.z);
      mk(pos.x, pos.y + (op.objPos ? 1.6 : 3), pos.z, op.alarm > 0 ? 'obj hot' : 'obj', op.icon, `${op.name} · ${Math.round(d)} m`, true);
    }
    if (charge) mk(charge.op.objPos.x, charge.op.objPos.y + 1, charge.op.objPos.z, 'obj hot', '!', `${charge.t.toFixed(1)} s`, true);
    for (const s of SUPPLY) { if (!s.active) continue; const d = s.pos.distanceTo(P.pos); if (d < 70) mk(s.pos.x, s.pos.y + 1.2, s.pos.z, 'sup', '+', d < 8 ? s.name : `${Math.round(d)} m`, false); }
    for (const e of enemies) {
      if (e.hp <= 0) continue;
      const p = e.g.position, d = Math.hypot(p.x - P.pos.x, p.z - P.pos.z);
      const hy = e.soldier ? 2.25 * e.g.scale.y : e.turret ? 2.5 : 1.2;
      if (e.soldier && e.st !== 'combat' && e.det > .04 && d < 60) mk(p.x, p.y + hy, p.z, e.det > .45 ? 'det warn' : 'det', '', '', false, e.det);
      else if (e.soldier && e.st === 'combat' && e.visible && d < 80) mk(p.x, p.y + hy, p.z, 'det hot', '!', '', false, 1);
      else if ((e.tagged || cityState.intel) && d < 110 && !e.turret) mk(p.x, p.y + hy, p.z, 'tag', '', '', false);
      else if (e.surv && e.surv.det > .05) mk(p.x, p.y + 1.5, p.z, 'det warn', '', '', false, e.surv.det);
    }
  }
  for (let i = mkUsed; i < mkPool.length; i++) if (!mkPool[i].hidden) mkPool[i].hidden = true;
}

/* --- panel de objetivos y estado --- */
function renderObjList() {
  const el = $('objlist'); if (!el) return;
  el.textContent = '';
  for (const op of OUTPOSTS) {
    const li = document.createElement('li'); li.className = op.done ? 'done' : '';
    const i = document.createElement('b'); i.textContent = op.icon;
    const t = document.createElement('span'); t.className = 't'; t.textContent = op.done ? op.full : op.task;
    const s = document.createElement('span'); s.className = 's'; s.textContent = op.effect;
    li.append(i, t, s); el.appendChild(li);
  }
}

/* --- mapa táctico (Tab) --- */
const bigmap = $('bigmap'), bctx = bigmap.getContext('2d');
function drawBigMap() {
  const W = bigmap.clientWidth, H = bigmap.clientHeight, dpr = Math.min(2, devicePixelRatio);
  if (bigmap.width !== W * dpr) { bigmap.width = W * dpr; bigmap.height = H * dpr; }
  const g = bctx; g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, W, H);
  const size = Math.min(W, H) - 80, k = size / (A * 2), ox = W / 2, oz = H / 2;
  const X = x => ox + x * k, Z = z => oz + z * k;
  g.fillStyle = 'rgba(8,12,14,.86)'; g.fillRect(0, 0, W, H);
  g.fillStyle = '#20282c'; g.fillRect(X(-A), Z(-A), A * 2 * k, A * 2 * k);
  for (const bxx of BC) for (const bz of BC) { g.fillStyle = '#323b40'; g.fillRect(X(bxx - 16), Z(bz - 16), 32 * k, 32 * k); }
  for (const b of boxes) {
    const h = b.max.y; if (h < 2.4 || b.max.x - b.min.x > 60 || b.max.z - b.min.z > 60) continue;
    g.fillStyle = h > 12 ? '#6d7880' : h > 5 ? '#55606a' : '#46505a';
    g.fillRect(X(b.min.x), Z(b.min.z), Math.max(1, (b.max.x - b.min.x) * k), Math.max(1, (b.max.z - b.min.z) * k));
  }
  g.font = '600 13px "Saira Condensed", system-ui, sans-serif'; g.textAlign = 'center';
  for (const op of OUTPOSTS) {
    const c = op.done ? '#5dffb0' : op.alarm > 0 ? '#ff3d5a' : '#ffb02e';
    g.strokeStyle = c; g.lineWidth = 2; g.setLineDash(op.done ? [] : [5, 4]); g.strokeRect(X(op.cx - 16), Z(op.cz - 16), 32 * k, 32 * k); g.setLineDash([]);
    g.fillStyle = c; g.fillText(`${op.icon} · ${op.full.toUpperCase()}`, X(op.cx), Z(op.cz - 18));
    if (!op.done) { g.fillStyle = 'rgba(230,237,232,.7)'; g.font = '12px system-ui, sans-serif'; g.fillText(op.task, X(op.cx), Z(op.cz + 20)); g.font = '600 13px "Saira Condensed", system-ui, sans-serif'; }
  }
  for (const s of SUPPLY) if (s.active) { g.fillStyle = '#5dffb0'; g.fillRect(X(s.pos.x) - 5, Z(s.pos.z) - 1.5, 10, 3); g.fillRect(X(s.pos.x) - 1.5, Z(s.pos.z) - 5, 3, 10); }
  for (const e of enemies) {
    if (e.hp <= 0) continue;
    const show = e.turret || e.surv || e.hunting || e.tagged || cityState.intel || (e.soldier && e.st === 'combat' && e.seeT < 3);
    if (!show) continue;
    g.fillStyle = e.turret ? (cityState.powered ? '#ffb02e' : '#55606a') : e.surv || e.hunting ? '#ff6a2e' : '#ff3d5a';
    g.beginPath(); g.arc(X(e.g.position.x), Z(e.g.position.z), e.kind === 'commander' ? 5 : 3, 0, Math.PI * 2); g.fill();
  }
  const sy = Math.sin(P.yaw), cy = Math.cos(P.yaw), px = X(P.pos.x), pz = Z(P.pos.z);
  g.fillStyle = '#e6ede8'; g.beginPath(); g.moveTo(px - sy * 9, pz - cy * 9); g.lineTo(px + cy * 5 + sy * 4, pz - sy * 5 + cy * 4); g.lineTo(px - cy * 5 + sy * 4, pz + sy * 5 + cy * 4); g.closePath(); g.fill();
  g.textAlign = 'left'; g.fillStyle = 'rgba(230,237,232,.6)'; g.font = '12px system-ui, sans-serif';
  g.fillText('Ámbar: puesto activo · Rojo: en alerta · Verde: liberado · + reabastecimiento', X(-A), Z(A) + 18);
}

/* --- actualización del modo ciudad --- */
function updateCity(dt, t) {
  cityState.time += dt;
  updateOutposts(dt);
  updateInteract(dt);
  updateCorpses(dt);
  updateAntennaFall(dt);
  // los soldiers no se atraviesan
  for (const e of enemies) {
    if (!e.soldier) continue;
    const p = e.g.position, dx = P.pos.x - p.x, dz = P.pos.z - p.z, d = Math.hypot(dx, dz);
    if (d < .75 && d > .001 && Math.abs(p.y - (P.pos.y - EYE)) < 1.5) { P.pos.x = p.x + dx / d * .75; P.pos.z = p.z + dz / d * .75; }
  }
}
