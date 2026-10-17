/* ---------- materiales extra y utilidades de modelado ---------- */
const wearTex = canvasTex(128, (g, s) => {
  g.fillStyle = '#9a9a9a'; g.fillRect(0, 0, s, s);
  for (let i = 0; i < 2600; i++) { const v = 110 + Math.random()*120 | 0; g.fillStyle = `rgb(${v},${v},${v})`; g.fillRect(Math.random()*s, Math.random()*s, 1 + Math.random()*2, 1); }
  for (let i = 0; i < 40; i++) { g.strokeStyle = `rgba(230,230,230,${.2 + Math.random()*.3})`; g.lineWidth = 1; g.beginPath(); const x = Math.random()*s, y = Math.random()*s; g.moveTo(x, y); g.lineTo(x + (Math.random()-.5)*30, y + (Math.random()-.5)*6); g.stroke(); }
}, 3);
wearTex.encoding = THREE.LinearEncoding;
const perfTex = canvasTex(128, (g, s) => {
  g.fillStyle = '#fff'; g.fillRect(0, 0, s, s); g.fillStyle = '#000';
  for (let y = 8; y < s; y += 16) for (let x = (y / 16 % 2) * 8 + 4; x < s; x += 16) { g.beginPath(); g.ellipse(x, y, 5, 4, 0, 0, Math.PI*2); g.fill(); }
}, 1);
perfTex.encoding = THREE.LinearEncoding; perfTex.repeat.set(4, 2);
Object.assign(gm, {
  metal: new THREE.MeshStandardMaterial({ color: 0x353c42, metalness: .85, roughness: .55, roughnessMap: wearTex }),
  steel: new THREE.MeshStandardMaterial({ color: 0x5b646b, metalness: 1, roughness: .32, roughnessMap: wearTex }),
  black: new THREE.MeshStandardMaterial({ color: 0x07090a, metalness: .3, roughness: .8 }),
  fde: new THREE.MeshStandardMaterial({ color: 0x5c4b37, metalness: .05, roughness: .72, roughnessMap: wearTex }),
  rubber: new THREE.MeshStandardMaterial({ color: 0x111314, metalness: 0, roughness: .95 }),
  perf: new THREE.MeshStandardMaterial({ color: 0x2a3035, metalness: .8, roughness: .45, alphaMap: perfTex, alphaTest: .5, side: THREE.DoubleSide })
});
gm.fde.envMapIntensity = .55;
gm.darkDS = gm.dark.clone(); gm.darkDS.side = THREE.DoubleSide;
gm.metalDS = gm.metal.clone(); gm.metalDS.side = THREE.DoubleSide;

// perfil lateral (u = hacia adelante, v = arriba) extruido a lo ancho del arma, con bisel
function tracePath(path, pts) {
  path.moveTo(pts[0][0], pts[0][1]);
  for (let i = 1; i < pts.length; i++) { const q = pts[i]; if (q[0] === 'q') path.quadraticCurveTo(q[1], q[2], q[3], q[4]); else path.lineTo(q[0], q[1]); }
  return path;
}
function EX(p, pts, width, x, m, o = {}) {
  const b = o.bevel ?? .004;
  const sh = tracePath(new THREE.Shape(), pts);
  for (const h of o.holes || []) sh.holes.push(tracePath(new THREE.Path(), h));
  const d = Math.max(.001, width - b*2);
  const geo = new THREE.ExtrudeGeometry(sh, { depth: d, bevelEnabled: b > 0, bevelThickness: b, bevelSize: b, bevelSegments: 2, curveSegments: 10 });
  geo.translate(0, 0, -d/2); geo.rotateY(Math.PI/2);
  const mesh = new THREE.Mesh(geo, m); mesh.position.x = x; p.add(mesh); return mesh;
}
function roundRect(path, cx, cy, w, h, r) {
  const x = cx - w/2, y = cy - h/2;
  path.moveTo(x + r, y); path.lineTo(x + w - r, y); path.quadraticCurveTo(x + w, y, x + w, y + r);
  path.lineTo(x + w, y + h - r); path.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  path.lineTo(x + r, y + h); path.quadraticCurveTo(x, y + h, x, y + h - r);
  path.lineTo(x, y + r); path.quadraticCurveTo(x, y, x + r, y); return path;
}
// marco visto de frente (x, y) con ventana, extruido hacia atrás: capuchas de mira
function FRAME(p, cy, w, h, hw, hh, depth, z, m) {
  const sh = roundRect(new THREE.Shape(), 0, cy, w, h, .008);
  sh.holes.push(roundRect(new THREE.Path(), 0, cy + .002, hw, hh, .005));
  const geo = new THREE.ExtrudeGeometry(sh, { depth, bevelEnabled: true, bevelThickness: .002, bevelSize: .002, bevelSegments: 1, curveSegments: 6 });
  geo.translate(0, 0, -depth/2);
  const mesh = new THREE.Mesh(geo, m); mesh.position.z = z; p.add(mesh); return mesh;
}
function OT(p, r, len, x, y, z, m) { const o = new THREE.Mesh(tubeGeo, m); o.scale.set(r, r, len); o.position.set(x, y, z); p.add(o); return o; }
function CY(p, r, len, x, y, z, m) { const o = C(p, r, len, x, y, z, m); o.rotation.x = Math.PI/2; return o; } // cilindro vertical
function decal(p, text, w, h, x, y, z, color = 'rgba(205,214,210,.75)') {
  const c = document.createElement('canvas'); c.width = 512; c.height = Math.round(512 * h / w);
  const g = c.getContext('2d'); g.fillStyle = color; g.font = `600 ${Math.round(c.height * .62)}px "Share Tech Mono", ui-monospace, monospace`; g.textBaseline = 'middle';
  g.fillText(text, 6, c.height / 2);
  const t = new THREE.CanvasTexture(c); t.encoding = THREE.sRGBEncoding; t.anisotropy = 4;
  const o = new THREE.Mesh(new THREE.PlaneGeometry(w, h), new THREE.MeshBasicMaterial({ map: t, transparent: true, depthWrite: false }));
  o.rotation.y = -Math.PI/2; o.position.set(x, y, z); p.add(o); return o;
}
function reticleMat(color) {
  const c = document.createElement('canvas'); c.width = c.height = 128; const g = c.getContext('2d');
  g.strokeStyle = color; g.fillStyle = color; g.lineWidth = 4; g.shadowColor = color; g.shadowBlur = 8;
  g.beginPath(); g.arc(64, 64, 44, 0, Math.PI*2); g.stroke();
  g.beginPath(); g.arc(64, 64, 5, 0, Math.PI*2); g.fill();
  for (const [x, y, w, h] of [[62, 4, 4, 14], [62, 110, 4, 14], [4, 62, 14, 4], [110, 62, 14, 4]]) g.fillRect(x, y, w, h);
  const t = new THREE.CanvasTexture(c);
  return new THREE.MeshBasicMaterial({ map: t, transparent: true, blending: THREE.AdditiveBlending, depthWrite: false });
}

// PR-7: carabina modular, receptor de aluminio, guardamanos facetado y culata FDE, mira holográfica
function buildRifle() {
  const g = new THREE.Group(), r = {};
  EX(g, [[-.1, 0], [.19, 0], [.19, .046], [.172, .062], [-.1, .062]], .064, 0, gm.metal);
  B(g, .05, .012, .30, 0, .068, -.045, gm.dark);
  for (let i = 0; i < 12; i++) B(g, .054, .006, .01, 0, .077, .1 - i*.025, gm.black);
  EX(g, [[.18, -.03], [.46, -.03], [.476, -.014], [.476, .046], [.46, .058], [.18, .058]], .072, 0, gm.fde, { bevel: .006 });
  B(g, .046, .008, .27, 0, .066, -.32, gm.dark);
  for (let i = 0; i < 9; i++) B(g, .05, .005, .009, 0, .072, -.2 - i*.03, gm.black);
  for (const sx of [-1, 1]) for (let i = 0; i < 3; i++) B(g, .004, .012, .05, sx*.0395, .012, -.24 - i*.075, gm.black);
  B(g, .002, .005, .22, -.0405, -.016, -.32, gm.amber);
  EX(g, [[-.1, 0], [.17, 0], [.17, -.024], [.125, -.034], [.125, -.062], [.05, -.062], [.05, -.03], [-.1, -.03]], .058, 0, gm.metal);
  EX(g, [[-.02, -.03], [-.062, -.142], [-.106, -.142], [-.112, -.128], [-.074, -.03]], .044, 0, gm.poly, { bevel: .006 });
  for (let i = 0; i < 4; i++) B(g, .05, .003, .016, 0, -.06 - i*.02, .052 + i*.009, gm.black);
  EX(g, [[-.016, -.03], [.052, -.03], [.042, -.074], [-.012, -.074]], .012, 0, gm.dark,
    { bevel: .002, holes: [[[-.006, -.036], [.04, -.036], [.033, -.066], [-.004, -.066]]] });
  EX(g, [[.012, -.034], [.02, -.034], [.017, -.058], [.009, -.057]], .006, 0, gm.steel, { bevel: 0 });
  r.mag = new THREE.Group(); g.add(r.mag);
  EX(r.mag, [[.056, -.056], [.12, -.056], ['q', .136, -.13, .16, -.198], [.094, -.212], ['q', .074, -.13, .056, -.056]], .046, 0, gm.poly, { bevel: .005 });
  for (let i = 0; i < 4; i++) B(r.mag, .052, .004, .05, 0, -.09 - i*.028, -.088 - i*.011, gm.black);
  EX(r.mag, [[.092, -.206], [.164, -.193], [.168, -.206], [.094, -.221]], .052, 0, gm.amber, { bevel: .002 });
  C(g, .016, .2, 0, .026, .2, gm.steel);
  EX(g, [[-.12, .042], [-.36, .046], [-.372, .036], [-.372, -.088], [-.34, -.094], [-.2, -.032], [-.12, -.012]], .046, 0, gm.fde,
    { bevel: .006, holes: [[[-.2, .026], [-.315, .029], [-.315, -.05], [-.24, -.022]]] });
  EX(g, [[-.372, .048], [-.388, .048], [-.388, -.094], [-.372, -.094]], .05, 0, gm.rubber, { bevel: .004 });
  C(g, .011, .14, 0, .012, -.53, gm.steel);
  B(g, .032, .034, .022, 0, .012, -.49, gm.dark);
  C(g, .019, .06, 0, .012, -.6, gm.dark);
  for (const sx of [-1, 1]) for (let k = 0; k < 3; k++) B(g, .004, .009, .011, sx*.0192, .012, -.58 - k*.017, gm.black);
  B(g, .072, .012, .014, 0, .052, .093, gm.dark);
  B(g, .002, .02, .07, .0325, .03, -.06, gm.black);
  B(g, .006, .01, .018, -.032, -.012, .03, gm.amber);
  B(g, .048, .012, .074, 0, .082, -.062, gm.dark);
  FRAME(g, .114, .056, .052, .044, .036, .062, -.068, gm.metal);
  B(g, .044, .036, .002, 0, .116, -.096, gm.glass);
  const ret = new THREE.Mesh(new THREE.PlaneGeometry(.026, .026), reticleMat('#ff3d5a')); ret.position.set(0, .114, -.092); g.add(ret);
  r.dot = ret;
  C(g, .006, .01, .031, .118, -.05, gm.steel).rotation.y = Math.PI/2;
  decal(g, 'PR-7 · PULSO · 4.6', .11, .014, -.0335, .03, -.05);
  return [g, r];
}
// KS-12: escopeta de corredera, guardamano perforado, cartuchera lateral, mira de anillo y fibra óptica
function buildShotgun() {
  const g = new THREE.Group(), r = {};
  EX(g, [[-.12, -.035], [.14, -.035], [.14, .046], [.105, .056], [-.06, .056], ['q', -.112, .056, -.12, .02]], .064, 0, gm.metal, { bevel: .005 });
  B(g, .002, .022, .07, .0335, .015, -.02, gm.black);
  B(g, .006, .05, .12, -.036, .002, .0, gm.poly);
  for (let i = 0; i < 4; i++) {
    CY(g, .0095, .054, -.048, .004, .045 - i*.03, gm.shell);
    CY(g, .0099, .013, -.048, .036, .045 - i*.03, gm.brass);
  }
  C(g, .02, .48, 0, .02, -.38, gm.steel);
  const shieldGeo = new THREE.CylinderGeometry(.029, .029, .24, 20, 1, true, Math.PI/2, Math.PI); shieldGeo.rotateX(Math.PI/2);
  const shield = new THREE.Mesh(shieldGeo, gm.perf); shield.position.set(0, .02, -.3); g.add(shield);
  B(g, .012, .006, .17, 0, .046, -.535, gm.dark);
  for (let i = 0; i < 3; i++) B(g, .004, .006, .004, 0, .04, -.47 - i*.06, gm.dark);
  C(g, .017, .36, 0, -.025, -.32, gm.dark);
  C(g, .019, .02, 0, -.025, -.505, gm.steel);
  B(g, .03, .066, .016, 0, -.002, -.49, gm.metal);
  r.pump = new THREE.Group(); g.add(r.pump);
  EX(r.pump, [[.2, -.06], [.4, -.06], ['q', .42, -.046, .42, -.032], [.42, -.01], [.2, -.01], ['q', .19, -.034, .2, -.06]], .07, 0, gm.poly, { bevel: .006 });
  for (let i = 0; i < 7; i++) B(r.pump, .084, .046, .005, 0, -.035, -.225 - i*.025, gm.black);
  EX(g, [[-.12, .046], [-.25, .03], [-.42, .008], [-.432, 0], [-.432, -.098], [-.384, -.11], [-.2, -.068], [-.13, -.056], [-.104, -.138], [-.06, -.134], [-.058, -.035], [-.12, -.035]], .046, 0, gm.poly,
    { bevel: .006, holes: [[[-.22, .014], [-.37, .0], [-.37, -.07], [-.24, -.045]]] });
  EX(g, [[-.432, .008], [-.448, .008], [-.448, -.1], [-.432, -.1]], .05, 0, gm.rubber);
  for (const sx of [-1, 1]) B(g, .002, .005, .18, sx*.0242, .021, .29, gm.red);
  for (let i = 0; i < 4; i++) B(g, .048, .003, .014, 0, -.07 - i*.017, .09 + i*.008, gm.black);
  EX(g, [[-.04, -.035], [.03, -.035], [.02, -.078], [-.036, -.078]], .012, 0, gm.dark,
    { bevel: .002, holes: [[[-.03, -.04], [.018, -.04], [.012, -.07], [-.028, -.07]]] });
  EX(g, [[-.012, -.039], [-.004, -.039], [-.007, -.064], [-.015, -.063]], .006, 0, gm.steel, { bevel: 0 });
  B(g, .012, .03, .014, 0, .068, .06, gm.dark);
  T(g, .013, .0032, 0, .08, .06, gm.steel);
  B(g, .004, .028, .006, 0, .063, -.53, gm.dark);
  r.dot = new THREE.Mesh(new THREE.SphereGeometry(.0032, 10, 8), gm.red); r.dot.position.set(0, .08, -.53); g.add(r.dot);
  decal(g, 'KS-12 · DISPERSIÓN', .1, .013, -.0335, -.012, -.02);
  return [g, r];
}
// RP-3: riel de partículas, cuerpo facetado, horquilla de rieles, bobinas, cápsula de energía y mira tubular
function buildRail() {
  const g = new THREE.Group(), r = {};
  EX(g, [[-.14, -.04], [.26, -.04], [.3, -.014], [.3, .034], [.22, .06], [-.06, .06], [-.14, .044]], .074, 0, gm.metal, { bevel: .006 });
  EX(g, [[-.05, .06], [.2, .06], [.18, .076], [-.03, .076]], .05, 0, gm.dark, { bevel: .003 });
  for (let i = 0; i < 7; i++) B(g, .052, .004, .009, 0, .08, .02 - i*.028, gm.black);
  B(g, .003, .034, .26, -.0395, .004, -.07, gm.dark);
  B(g, .003, .005, .24, -.041, .02, -.07, gm.cyan);
  B(g, .003, .005, .12, -.041, -.012, -.01, gm.cyan);
  C(g, .019, .2, 0, -.062, -.06, gm.glass);
  r.core = C(g, .009, .19, 0, -.062, -.06, gm.cyan);
  for (const z of [.04, -.16]) C(g, .021, .016, 0, -.062, z, gm.steel);
  for (const sx of [-1, 1]) {
    EX(g, [[.26, -.03], [.72, -.012], [.72, .012], [.26, .04]], .016, sx*.031, gm.steel, { bevel: .003 });
    B(g, .003, .008, .44, sx*.021, 0, -.49, gm.cyan);
  }
  r.coils = [];
  for (let i = 0; i < 6; i++) r.coils.push(T(g, .044, .007, 0, 0, -.33 - i*.066, gm.cyan));
  B(g, .078, .018, .022, 0, 0, -.71, gm.dark);
  EX(g, [[-.03, -.04], [-.066, -.15], [-.112, -.15], [-.082, -.04]], .046, 0, gm.poly, { bevel: .006 });
  EX(g, [[-.0, -.04], [.06, -.04], [.05, -.08], [-.0, -.08]], .012, 0, gm.dark, { bevel: .002, holes: [[[.008, -.045], [.048, -.045], [.042, -.073], [.008, -.073]]] });
  for (let k = 0; k < 3; k++) {
    C(g, .012, .055, -.05, -.022, .02 - k*.07, gm.dark);
    T(g, .0125, .002, -.05, -.022, .0 - k*.07, gm.cyan);
  }
  EX(g, [[-.14, .044], [-.38, .03], [-.392, .02], [-.392, -.09], [-.352, -.096], [-.14, -.03]], .044, 0, gm.dark,
    { bevel: .005, holes: [[[-.18, .022], [-.34, .014], [-.34, -.06], [-.2, -.02]]] });
  EX(g, [[-.392, .032], [-.406, .032], [-.406, -.096], [-.392, -.096]], .048, 0, gm.rubber);
  B(g, .02, .025, .03, 0, .072, -.02, gm.dark); B(g, .02, .025, .03, 0, .072, -.13, gm.dark);
  OT(g, .024, .2, 0, .096, -.07, gm.darkDS);
  OT(g, .031, .04, 0, .096, -.19, gm.metalDS);
  OT(g, .028, .025, 0, .096, .035, gm.metalDS);
  C(g, .009, .018, 0, .126, -.07, gm.steel).rotation.x = Math.PI/2;
  C(g, .009, .018, .03, .096, -.07, gm.steel).rotation.y = Math.PI/2;
  B(g, .046, .046, .002, 0, .096, -.205, gm.glass);
  r.dot = new THREE.Mesh(new THREE.SphereGeometry(.0018, 8, 6), gm.cyan); r.dot.position.set(0, .096, -.2); g.add(r.dot);
  decal(g, 'RP-3 · RIEL', .08, .014, -.0415, .04, -.05, 'rgba(95,216,255,.8)');
  return [g, r];
}
// LG-6: lanzagranadas revólver, placas laterales caladas, tambor estriado, culata plegable
function buildLauncher() {
  const g = new THREE.Group(), r = { drumTarget: 0 };
  OT(g, .042, .36, 0, .02, -.36, gm.darkDS);
  C(g, .036, .34, 0, .02, -.36, gm.black);
  for (const z of [-.24, -.36, -.48]) T(g, .044, .004, 0, .02, z, gm.steel);
  OT(g, .048, .03, 0, .02, -.535, gm.metalDS);
  B(g, .03, .01, .3, 0, .066, -.36, gm.dark);
  for (let i = 0; i < 8; i++) B(g, .034, .005, .008, 0, .073, -.24 - i*.032, gm.dark);
  for (const sx of [-1, 1]) EX(g, [[-.08, -.086], [.16, -.086], [.182, -.04], [.182, .072], [-.08, .072]], .01, sx*.076, gm.metal,
    { bevel: .002, holes: [[[-.045, -.062], [.105, -.062], [.105, .048], [-.045, .048]]] });
  B(g, .11, .012, .27, 0, .077, -.05, gm.dark);
  for (let i = 0; i < 8; i++) B(g, .114, .004, .01, 0, .084, .06 - i*.032, gm.black);
  B(g, .162, .012, .27, 0, -.09, -.05, gm.metal);
  r.drum = new THREE.Group(); r.drum.position.set(0, -.01, -.02); g.add(r.drum);
  C(r.drum, .07, .12, 0, 0, 0, gm.steel);
  for (let i = 0; i < 6; i++) {
    const a = i/6*Math.PI*2, a2 = a + Math.PI/6;
    C(r.drum, .017, .124, Math.cos(a)*.044, Math.sin(a)*.044, 0, gm.black);
    C(r.drum, .015, .012, Math.cos(a)*.044, Math.sin(a)*.044, -.058, gm.amber);
    const fl = B(r.drum, .014, .01, .1, Math.cos(a2)*.068, Math.sin(a2)*.068, 0, gm.black); fl.rotation.z = a2;
  }
  EX(g, [[.24, -.022], [.3, -.022], [.292, -.135], [.252, -.135]], .036, 0, gm.poly, { bevel: .006 });
  EX(g, [[-.03, -.09], [-.066, -.195], [-.112, -.195], [-.082, -.09]], .044, 0, gm.poly, { bevel: .006 });
  EX(g, [[-.012, -.09], [.05, -.09], [.04, -.13], [-.01, -.13]], .012, 0, gm.dark, { bevel: .002, holes: [[[-.003, -.095], [.04, -.095], [.034, -.124], [-.002, -.124]]] });
  for (const sx of [-1, 1]) B(g, .008, .012, .22, sx*.022, .03, .19, gm.steel);
  EX(g, [[-.29, .052], [-.312, .052], [-.312, -.082], [-.29, -.082]], .056, 0, gm.rubber, { bevel: .005 });
  B(g, .044, .012, .012, 0, -.03, .29, gm.steel);
  B(g, .006, .014, .006, 0, .089, -.02, gm.dark);
  T(g, .012, .003, 0, .1, -.02, gm.steel);
  B(g, .004, .036, .006, 0, .081, -.5, gm.dark);
  r.dot = new THREE.Mesh(new THREE.SphereGeometry(.003, 8, 6), gm.amber); r.dot.position.set(0, .1, -.5); g.add(r.dot);
  decal(g, 'LG-6 · 40 MM', .09, .014, -.0825, .05, -.03);
  return [g, r];
}

const gun = new THREE.Group();
gunCam.add(gun);
const models = [buildRifle(), buildShotgun(), buildRail(), buildLauncher()];
for (const [m] of models) gun.add(m);
const flash = new THREE.Mesh(new THREE.PlaneGeometry(.42, .42), new THREE.MeshBasicMaterial({ map: glowTex, transparent: true, blending: THREE.AdditiveBlending, depthWrite: false }));
flash.visible = false; gun.add(flash);
const flashLight = new THREE.PointLight(0xffb040, 0, 2, 2); gun.add(flashLight);
const worldFlash = new THREE.PointLight(0xffb040, 0, 10, 2); worldFlash.position.set(0, 0, -1); camera.add(worldFlash);
const GUN_BASE = new V3(.17, -.175, -.39);
gun.position.copy(GUN_BASE);
function applyGunModel(w) {
  const W = WEAPONS[w];
  models.forEach(([m], i) => { m.visible = i === w; });
  flash.position.set(0, W.muzzle[0], W.muzzle[1] - .02);
  flashLight.position.copy(flash.position);
  flashLight.color.setHex(W.color); worldFlash.color.setHex(W.color);
  flash.material.color.setHex(w === 2 ? 0x9fe9ff : 0xffffff);
  flash.scale.setScalar(w === 1 || w === 3 ? 1.5 : 1);
}
applyGunModel(0);

