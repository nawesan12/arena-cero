/* ---------- enemigos ---------- */
const eGeoShell = new THREE.OctahedronGeometry(.7, 0);
const eGeoCore = new THREE.SphereGeometry(.3, 12, 8);
const eGeoRing = new THREE.TorusGeometry(.98, .06, 6, 32);
const eGeoKami = new THREE.ConeGeometry(.42, 1.1, 4); eGeoKami.rotateX(Math.PI/2);
const eGeoKamiFin = new THREE.BoxGeometry(1.2, .04, .3);
const eMat = {
  shell: new THREE.MeshStandardMaterial({ color: 0x1b2226, metalness: .65, roughness: .35, flatShading: true }),
  shellH: new THREE.MeshStandardMaterial({ color: 0x2b2418, metalness: .65, roughness: .35, flatShading: true }),
  shellK: new THREE.MeshStandardMaterial({ color: 0x3a1418, metalness: .5, roughness: .4, flatShading: true, emissive: 0x401010 }),
  flash: new THREE.MeshBasicMaterial({ color: 0xffffff }),
  red: new THREE.MeshBasicMaterial({ color: 0xff3d5a }),
  amber: new THREE.MeshBasicMaterial({ color: 0xffb02e }),
  hot: new THREE.MeshBasicMaterial({ color: 0xfff0e0 }),
  orange: new THREE.MeshBasicMaterial({ color: 0xff6a2e })
};
let hitTargets = [], targetsDirty = true, bossRef = null;
function spawnEnemy(kind) {
  const g = new THREE.Group();
  const heavy = kind === 'h', kami = kind === 'k', boss = kind === 'b';
  let shell, ring = null, ring2 = null, base, color, core = null;
  if (kami) {
    base = eMat.shellK; color = 0xff6a2e;
    shell = new THREE.Mesh(eGeoKami, base);
    const fin = new THREE.Mesh(eGeoKamiFin, eMat.orange); fin.position.z = .25;
    g.add(shell, fin);
  } else {
    base = heavy || boss ? eMat.shellH : eMat.shell; color = heavy ? 0xffb02e : 0xff3d5a;
    const col = heavy ? eMat.amber : eMat.red;
    shell = new THREE.Mesh(eGeoShell, base); shell.scale.set(1, 1.15, 1);
    core = new THREE.Mesh(eGeoCore, boss ? eMat.hot : col); core.position.z = .55;
    ring = new THREE.Mesh(eGeoRing, col);
    g.add(shell, core, ring);
    if (boss) { ring2 = new THREE.Mesh(eGeoRing, eMat.amber); ring2.scale.setScalar(1.3); g.add(ring2); }
  }
  const s = boss ? 2.8 : heavy ? 1.6 : 1; g.scale.setScalar(s);
  let x, z, tries = 0, dd;
  do { x = (Math.random()*2-1)*(A-6); z = (Math.random()*2-1)*(A-6); dd = Math.hypot(x - P.pos.x, z - P.pos.z); tries++; _a.set(x, 3, z); }
  while ((dd < Math.min(22, A*.6) || dd > 48 || boxAt(_a)) && tries < 60);
  g.position.set(x, boss ? 12 : 9, z);
  scene.add(g);
  const hp = boss ? 40 + wave*5 : heavy ? 16 : kami ? 2 : 4;
  const e = { g, shell, ring, ring2, base, heavy, kami, boss, color, hp, maxHp: hp, vel: new V3(), phase: Math.random()*10,
    speed: boss ? 2.4 : kami ? 7.5 + Math.min(wave*.2, 2.5) : (heavy ? 3.2 : 5) + Math.min(wave*.2, 2.5),
    fireCd: 2 + Math.random()*1.5, flash: 0, radius: (kami ? .6 : .9)*s, value: boss ? 3000 : heavy ? 500 : kami ? 150 : 100 };
  shell.userData.enemy = e; e.hitMeshes = [shell];
  e.flank = (!heavy && !kami && !boss && Math.random() < .45) ? (Math.random() < .5 ? -1 : 1) : 0;
  if (core) { core.userData.enemy = e; core.userData.crit = true; e.hitMeshes.push(core); }
  enemies.push(e); targetsDirty = true;
  if (boss) bossRef = e;
  burst(g.position, color, boss ? 40 : 14, boss ? 9 : 5);
}
const shotGeo = new THREE.SphereGeometry(.16, 8, 6);
const glowGeo = new THREE.SphereGeometry(.34, 8, 6);
const shotCore = new THREE.MeshBasicMaterial({ color: 0xfff0e0 });
const glowRed = new THREE.MeshBasicMaterial({ color: 0xff3d5a, transparent: true, opacity: .45, blending: THREE.AdditiveBlending, depthWrite: false });
const glowAmber = glowRed.clone(); glowAmber.color.setHex(0xffb02e);
const shotPool = [];
function enemyFire(e, dir) {
  let m = shotPool.pop();
  if (!m) { m = new THREE.Mesh(shotGeo, shotCore); m.add(new THREE.Mesh(glowGeo, glowRed)); }
  m.children[0].material = e.heavy ? glowAmber : glowRed;
  m.position.copy(e.g.position).addScaledVector(dir, e.radius + .2); scene.add(m);
  const sp = e.boss ? 9 : Math.min(11 + wave*.35, 17) * (e.heavy ? .85 : 1);
  shots.push({ m, v: dir.clone().multiplyScalar(sp), life: e.boss ? 7 : 5, dmg: e.boss ? 10 : e.heavy ? 11 : 7, from: e.g.position.clone() });
}
function removeShot(i) { const s = shots[i]; scene.remove(s.m); shotPool.push(s.m); shots[i] = shots[shots.length-1]; shots.pop(); }
function damageEnemy(e, dmg, dir, crit = false) {
  if (e.hp <= 0) return false;
  queueNum(e, Math.min(dmg, e.hp), crit);
  e.hp -= dmg; e.flash = .07;
  if ((e.soldier || e.turret) && e.hp > 0) { // herido: entra en combate y avisa a su puesto
    e.st = 'combat'; e.lastSeen.copy(_body); e.seeT = 0; if (e.soldier) e.react = Math.min(e.react, .3);
    raiseAlarm(e.op, _body);
  }
  if (dir) e.vel.addScaledVector(dir, (e.boss ? .15 : e.heavy ? .8 : 2.5) * Math.min(dmg, 3));
  if (e.hp <= 0) { killEnemy(e); return true; }
  return false;
}
function killEnemy(e) {
  if (e.soldier || e.turret) { cityKill(e); return; }
  const i = enemies.indexOf(e); if (i < 0) return;
  enemies.splice(i, 1); scene.remove(e.g); targetsDirty = true;
  if (e.spot) { scene.remove(e.spot); e.spot = null; }
  burst(e.g.position, e.color, e.heavy ? 50 : 28, 9, .8);
  burst(e.g.position, 0x1b2226, 12, 6, 1);
  sfx.kill(); kills++;
  combo = comboT > 0 ? Math.min(combo + 1, 5) : 1; comboT = 3;
  if (combo > 1) { hud.combo.classList.add('pop'); setTimeout(() => hud.combo.classList.remove('pop'), 100); }
  score += e.value * combo;
  if (e.kami) explode(e.g.position, 3.2, 3, 0, false); // reacción en cadena contra otros drones
  if (P.leech) P.hp = Math.min(P.maxHp, P.hp + P.leech);
  if (isCity()) return;
  if (e.boss) {
    bossRef = null; explode(e.g.position, 6, 4, 0, true);
    spawnOrb(e.g.position); _c.copy(e.g.position); _c.x += 2.5; spawnOrb(_c, 'od');
    banner('COLOSO DERRIBADO', `+${(e.value * combo).toLocaleString('es-AR')} puntos`);
  } else if (e.heavy || Math.random() < (e.kami ? .1 : .3)) spawnOrb(e.g.position);
  else if (Math.random() < .05) spawnOrb(e.g.position, 'od');
}
const orbGeo = new THREE.IcosahedronGeometry(.32, 0);
const orbInGeo = new THREE.SphereGeometry(.14, 8, 6);
const orbMatW = new THREE.MeshBasicMaterial({ color: 0x5dffb0, wireframe: true });
const orbMat = new THREE.MeshBasicMaterial({ color: 0x5dffb0 });
const orbMatWA = new THREE.MeshBasicMaterial({ color: 0xffb02e, wireframe: true });
const orbMatA = new THREE.MeshBasicMaterial({ color: 0xffb02e });
function spawnOrb(p, type = 'hp') {
  const od = type === 'od';
  const m = new THREE.Mesh(orbGeo, od ? orbMatWA : orbMatW); m.add(new THREE.Mesh(orbInGeo, od ? orbMatA : orbMat));
  if (od) m.scale.setScalar(1.25);
  m.position.set(p.x, 0, p.z); m.userData.baseY = groundAt(p.x, p.z, 99) + .8;
  scene.add(m); orbs.push({ m, life: 20, type });
}

