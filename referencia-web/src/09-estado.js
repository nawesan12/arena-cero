/* ---------- estado ---------- */
const EYE = 1.6, R = .4, CROUCH_DROP = .65;
const P = { pos: new V3(), vel: new V3(), yaw: 0, pitch: 0, onGround: true, hp: 100, ammo: [24, 6, 4], w: 0, reload: 0, fireCd: 0,
  bob: 0, recoil: 0, aimT: 0, jumps: 0, swayX: 0, swayY: 0, crouchT: 0, slide: 0, slideCd: 0, swapT: 0, nades: 2, nadeCd: 0, shake: 0 };
const BASE_STATS = () => ({ maxHp: 100, dmgMul: 1, reloadMul: 1, magMul: 1, speedMul: 1, regenMul: 1, leech: 0, maxNades: 3, maxJumps: 2,
  dashCdMul: 1, critMul: 2, overdrive: 0, dashCd: 0, dashT: 0, dashReq: false, lastHurt: 0, hook: null, hookCd: 0, zoneAcc: 0 });
Object.assign(P, BASE_STATS());
const PERKS = [
  { id: 'hp', name: 'Blindaje', desc: '+25 de integridad máxima y reparación completa.', apply() { P.maxHp += 25; P.hp = P.maxHp; } },
  { id: 'dmg', name: 'Munición perforante', desc: '+25% de daño con todas las armas.', apply() { P.dmgMul += .25; } },
  { id: 'reload', name: 'Manos rápidas', desc: 'Recargás un 30% más rápido.', apply() { P.reloadMul *= .7; } },
  { id: 'mag', name: 'Cargador extendido', desc: '+50% de capacidad en todas las armas.', apply() { P.magMul += .5; P.ammo = WEAPONS.map((_, i) => magOf(i)); } },
  { id: 'speed', name: 'Servos', desc: '+12% de velocidad de movimiento.', apply() { P.speedMul += .12; } },
  { id: 'regen', name: 'Nanorreparación', desc: 'Regenerás integridad el doble de rápido.', apply() { P.regenMul *= 2; } },
  { id: 'leech', name: 'Absorción', desc: 'Cada baja repara 4 de integridad.', apply() { P.leech += 4; } },
  { id: 'nade', name: 'Bandolera', desc: '+1 granada máxima y recarga completa.', apply() { P.maxNades++; P.nades = P.maxNades; } },
  { id: 'jump', name: 'Propulsores', desc: 'Un salto extra en el aire.', apply() { P.maxJumps++; } },
  { id: 'dash', name: 'Impulso', desc: 'El dash recarga un 40% más rápido.', apply() { P.dashCdMul *= .6; } },
  { id: 'crit', name: 'Puntería fina', desc: 'Los disparos al núcleo hacen ×3 en vez de ×2.', once: true, apply() { P.critMul = 3; } }
];
let perksTaken = [], perkChoices = [];
function showPerks() {
  state = 'perk'; firing = aiming = false;
  const pool = PERKS.filter(p => !(p.once && perksTaken.includes(p.id)));
  perkChoices = [];
  while (perkChoices.length < 3 && pool.length) perkChoices.push(pool.splice(Math.floor(Math.random()*pool.length), 1)[0]);
  $('cards').innerHTML = perkChoices.map((p, i) => {
    const n = perksTaken.filter(x => x === p.id).length;
    return `<button class="card-p" data-i="${i}" id="perk${i}"><span class="k">${i + 1}${n ? ` · nivel ${n + 1}` : ''}</span><span class="n">${p.name}</span><span class="d">${p.desc}</span></button>`;
  }).join('');
  $('perks').hidden = false; sfx.perk();
}
function pickPerk(i) {
  if (state !== 'perk' || !perkChoices[i]) return;
  const pk = perkChoices[i]; pk.apply(); perksTaken.push(pk.id);
  if (pendingLevel) { pendingLevel = false; setTimeout(nextLevel, 0); }
  $('perks').hidden = true; state = 'playing'; clock.getDelta();
  banner(pk.name.toUpperCase(), pk.desc); sfx.pickup();
  if (!isTouch && !document.pointerLockElement && renderer.domElement.requestPointerLock) {
    try { const r = renderer.domElement.requestPointerLock(); if (r && r.catch) r.catch(() => {}); } catch (e) {}
  }
}
let state = 'menu', lockedOnce = false;
let wave = 0, score = 0, kills = 0, intermission = 0, spawnQueue = [], hurtFlash = 0, combo = 0, comboT = 0;
let sens = 1.8; try { sens = +localStorage.getItem('arena-cero-sens') || 1.8; } catch (e) {}
let best = 0; try { best = +localStorage.getItem('arena-cero-best') || 0; } catch (e) {}
const enemies = [], shots = [], orbs = [], nades = [];
const keys = {}; let firing = false, semiLock = false, aiming = false, aimToggle = false, crouchToggle = false;
const touchMove = { x: 0, y: 0 };

const _body = new V3(), _d = new V3(), _aim = new V3(), _a = new V3(), _b = new V3(), _c = new V3(), UP = new V3(0, 1, 0);

