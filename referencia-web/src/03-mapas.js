/* ---------- mapas ---------- */
const STEP = .34;
let A = 60, LIM = A - 1.5, MAP = null, mapId = 'deposito', floor = null;
const boxes = [];
let staticObjs = [], zones = [], scrollMats = [];
const corrTex = canvasTex(256, (g, s) => {
  g.fillStyle = '#d8d8d8'; g.fillRect(0, 0, s, s);
  for (let x = 0; x < s; x += 16) { const gr = g.createLinearGradient(x, 0, x + 16, 0); gr.addColorStop(0, '#9a9a9a'); gr.addColorStop(.5, '#f2f2f2'); gr.addColorStop(1, '#8a8a8a'); g.fillStyle = gr; g.fillRect(x, 0, 16, s); }
  for (let i = 0; i < 1800; i++) { g.fillStyle = `rgba(60,30,10,${Math.random()*.25})`; g.fillRect(Math.random()*s, Math.random()*s, 2 + Math.random()*6, 2); }
  g.strokeStyle = '#555'; g.lineWidth = 8; g.strokeRect(4, 4, s-8, s-8);
});
const concreteTex = canvasTex(256, (g, s) => {
  g.fillStyle = '#3a4246'; g.fillRect(0, 0, s, s);
  for (let i = 0; i < 4000; i++) { const v = 45 + Math.random()*30 | 0; g.fillStyle = `rgba(${v},${v+5},${v+8},.4)`; g.fillRect(Math.random()*s, Math.random()*s, 2, 2); }
  g.fillStyle = 'rgba(0,0,0,.25)'; g.fillRect(0, s-10, s, 10);
});
const lavaTex = canvasTex(256, (g, s) => {
  g.fillStyle = '#c2360c'; g.fillRect(0, 0, s, s);
  for (let i = 0; i < 260; i++) {
    const x = Math.random()*s, y = Math.random()*s, r = 6 + Math.random()*26;
    const gr = g.createRadialGradient(x, y, 0, x, y, r);
    gr.addColorStop(0, Math.random() < .5 ? 'rgba(255,214,90,.9)' : 'rgba(255,140,30,.8)'); gr.addColorStop(1, 'rgba(160,30,0,0)');
    g.fillStyle = gr; g.fillRect(x - r, y - r, r*2, r*2);
  }
  for (let i = 0; i < 40; i++) { g.strokeStyle = 'rgba(40,8,0,.55)'; g.lineWidth = 2 + Math.random()*4; g.beginPath(); g.moveTo(Math.random()*s, Math.random()*s); g.bezierCurveTo(Math.random()*s, Math.random()*s, Math.random()*s, Math.random()*s, Math.random()*s, Math.random()*s); g.stroke(); }
}, 2);
const windowTex = canvasTex(128, (g, s) => {
  g.fillStyle = '#0a0e18'; g.fillRect(0, 0, s, s);
  for (let y = 4; y < s; y += 10) for (let x = 4; x < s; x += 8) {
    const r = Math.random();
    g.fillStyle = r < .22 ? '#ffd27a' : r < .3 ? '#9fc7ff' : '#141a28'; g.fillRect(x, y, 5, 6);
  }
});
const matWall = new THREE.MeshStandardMaterial({ color: 0x20272b, roughness: .85 });
const matPillar = new THREE.MeshStandardMaterial({ color: 0x2a3236, roughness: .7, metalness: .2 });
const matCrate = new THREE.MeshStandardMaterial({ map: crateTex, roughness: .8 });
const matConcrete = new THREE.MeshStandardMaterial({ map: concreteTex, roughness: .9 });
const matContainers = [0x8a4528, 0x2a5a63, 0x4a5662, 0x6e6a3a].map(c => new THREE.MeshStandardMaterial({ map: corrTex, color: c, roughness: .6, metalness: .35 }));
const matStrip = new THREE.MeshBasicMaterial({ color: 0xffb02e });
const matStripW = new THREE.MeshBasicMaterial({ color: 0xdfe8e4 });
const matStripR = new THREE.MeshBasicMaterial({ color: 0xff3d5a });
const matLava = new THREE.MeshBasicMaterial({ map: lavaTex });
const matSteel = new THREE.MeshStandardMaterial({ color: 0x3b4248, metalness: .7, roughness: .4 });
const matTank = new THREE.MeshStandardMaterial({ color: 0x55606a, metalness: .5, roughness: .5 });
const matWindows = [1, 2, 3].map(r => { const t = windowTex.clone(); t.needsUpdate = true; t.repeat.set(r, r*2); return new THREE.MeshBasicMaterial({ map: t }); });
const floorMat = new THREE.MeshStandardMaterial({ map: floorTex, roughness: .92 });
const boxGeo = new THREE.BoxGeometry(1, 1, 1);
const tankGeo = new THREE.CylinderGeometry(1.6, 1.6, 4, 20);

function track(m) { scene.add(m); staticObjs.push(m); return m; }
/* Geometría estática por lotes: cada caja se acumula por material y al terminar el mapa se funde
   en una sola malla por material (pocas llamadas de dibujo aunque haya miles de cajas). */
let batches = new Map();
function addVisual(x, y, z, w, h, d, mat, shadow = true) {
  const key = mat.uuid + (shadow ? 's' : 'n');
  let b = batches.get(key); if (!b) { b = { mat, shadow, list: [] }; batches.set(key, b); }
  b.list.push(x, y, z, w, h, d);
}
function flushBatches() {
  const pos = boxGeo.attributes.position, nor = boxGeo.attributes.normal, uv = boxGeo.attributes.uv, idx = boxGeo.index, n = pos.count, ni = idx.count;
  for (const b of batches.values()) {
    const L = b.list, cnt = L.length / 6, wuv = b.mat.userData.worldUV || 0;
    const P = new Float32Array(cnt*n*3), N = new Float32Array(cnt*n*3), U = new Float32Array(cnt*n*2), I = new Uint32Array(cnt*ni);
    for (let k = 0; k < cnt; k++) {
      const x = L[k*6], y = L[k*6+1], z = L[k*6+2], w = L[k*6+3], h = L[k*6+4], d = L[k*6+5];
      for (let i = 0; i < n; i++) {
        const o = (k*n + i)*3, px = pos.getX(i)*w + x, py = pos.getY(i)*h + y, pz = pos.getZ(i)*d + z;
        const nx = nor.getX(i), ny = nor.getY(i), nz = nor.getZ(i);
        P[o] = px; P[o+1] = py; P[o+2] = pz; N[o] = nx; N[o+1] = ny; N[o+2] = nz;
        const u = (k*n + i)*2;
        if (wuv) { // UV en metros: las fachadas repiten ventanas por piso sin estirarse
          if (Math.abs(nx) > .5) { U[u] = pz / wuv; U[u+1] = py / wuv; }
          else if (Math.abs(nz) > .5) { U[u] = px / wuv; U[u+1] = py / wuv; }
          else { U[u] = px / wuv; U[u+1] = pz / wuv; }
        } else { U[u] = uv.getX(i); U[u+1] = uv.getY(i); }
      }
      for (let j = 0; j < ni; j++) I[k*ni + j] = idx.getX(j) + k*n;
    }
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.BufferAttribute(P, 3)); g.setAttribute('normal', new THREE.BufferAttribute(N, 3)); g.setAttribute('uv', new THREE.BufferAttribute(U, 2));
    g.setIndex(new THREE.BufferAttribute(I, 1)); g.computeBoundingSphere();
    const m = new THREE.Mesh(g, b.mat); m.castShadow = m.receiveShadow = b.shadow; m.matrixAutoUpdate = false; track(m);
  }
  batches = new Map();
}
/* Grilla espacial de colisionadores (celdas de 8 m). Las cajas pueden empezar en altura (pisos, losas). */
const CELL = 8, grid = new Map(); let gstamp = 1;
const gkey = (ix, iz) => (ix + 512) * 1024 + (iz + 512);
function gridAdd(b) {
  const x0 = Math.floor(b.min.x / CELL), x1 = Math.floor(b.max.x / CELL), z0 = Math.floor(b.min.z / CELL), z1 = Math.floor(b.max.z / CELL);
  for (let ix = x0; ix <= x1; ix++) for (let iz = z0; iz <= z1; iz++) { const k = gkey(ix, iz); let c = grid.get(k); if (!c) grid.set(k, c = []); c.push(b); }
}
function gridQuery(minx, minz, maxx, maxz, out) {
  out.length = 0; gstamp++;
  const x0 = Math.floor(minx / CELL), x1 = Math.floor(maxx / CELL), z0 = Math.floor(minz / CELL), z1 = Math.floor(maxz / CELL);
  for (let ix = x0; ix <= x1; ix++) for (let iz = z0; iz <= z1; iz++) {
    const c = grid.get(gkey(ix, iz)); if (!c) continue;
    for (let i = 0; i < c.length; i++) { const b = c[i]; if (b.st !== gstamp) { b.st = gstamp; out.push(b); } }
  }
  return out;
}
function addCollider(x, z, w, h, d, y0 = 0) {
  const b = { min: new V3(x-w/2, y0, z-d/2), max: new V3(x+w/2, y0+h, z+d/2), st: 0 };
  boxes.push(b); gridAdd(b); return b;
}
function addBox(x, z, w, h, d, mat) { addVisual(x, h/2, z, w, h, d, mat); addCollider(x, z, w, h, d); }
function removeCollider(b) {
  const i = boxes.indexOf(b); if (i >= 0) boxes.splice(i, 1);
  const x0 = Math.floor(b.min.x / CELL), x1 = Math.floor(b.max.x / CELL), z0 = Math.floor(b.min.z / CELL), z1 = Math.floor(b.max.z / CELL);
  for (let ix = x0; ix <= x1; ix++) for (let iz = z0; iz <= z1; iz++) { const c = grid.get(gkey(ix, iz)); if (c) { const j = c.indexOf(b); if (j >= 0) c.splice(j, 1); } }
}
function addBlock(x, y0, z, w, h, d, mat, shadow = true) { addVisual(x, y0 + h/2, z, w, h, d, mat, shadow); return addCollider(x, z, w, h, d, y0); }
function addStrip(x, y, z, w, h, d, mat = matStrip) { addVisual(x, y, z, w, h, d, mat, false); }
function addStairs(ex, ez, dx, dz, width, topH, mat) {
  const n = Math.round(topH / STEP), depth = .75;
  for (let k = 1; k < n; k++) {
    const h = topH - k*STEP, cx = ex + dx*(k - .5)*depth, cz = ez + dz*(k - .5)*depth;
    addBox(cx, cz, dx ? depth : width, h, dz ? depth : width, mat);
  }
}
function addContainer(x, z, alongX, stack, ci) {
  const w = alongX ? 6.2 : 2.6, d = alongX ? 2.6 : 6.2, h = 2.7;
  for (let s = 0; s < stack; s++) addVisual(x, h/2 + s*h, z, w, h, d, matContainers[(ci + s) % matContainers.length]);
  addCollider(x, z, w, h*stack, d);
}
function addPerimeter(h, stripY) {
  addBox(0, -A, A*2+2, h, 2, matWall); addBox(0, A, A*2+2, h, 2, matWall);
  addBox(-A, 0, 2, h, A*2+2, matWall); addBox(A, 0, 2, h, A*2+2, matWall);
  if (stripY) {
    addStrip(0, stripY, -A+1.05, A*2, .14, .1); addStrip(0, stripY, A-1.05, A*2, .14, .1);
    addStrip(-A+1.05, stripY, 0, .1, .14, A*2); addStrip(A-1.05, stripY, 0, .1, .14, A*2);
  }
}
function addLava(x, z, w, d, dps = 22) {
  const m = new THREE.Mesh(new THREE.PlaneGeometry(w, d), matLava);
  m.rotation.x = -Math.PI/2; m.position.set(x, .02, z); track(m);
  addStrip(x, .03, z - d/2 - .1, w + .4, .02, .2); addStrip(x, .03, z + d/2 + .1, w + .4, .02, .2);
  addStrip(x - w/2 - .1, .03, z, .2, .02, d); addStrip(x + w/2 + .1, .03, z, .2, .02, d);
  zones.push({ x, z, w, d, dps });
}

/* Depósito: complejo industrial abierto */
function buildDeposito() {
  addPerimeter(8, 6.4);
  const PH = STEP*3;
  addBox(0, 0, 12, PH, 12, matConcrete);
  addStairs(0, -6, 0, -1, 5, PH, matConcrete); addStairs(0, 6, 0, 1, 5, PH, matConcrete);
  addStairs(-6, 0, -1, 0, 5, PH, matConcrete); addStairs(6, 0, 1, 0, 5, PH, matConcrete);
  addBox(-4.2, -4.2, 2, PH + 1.1, 2, matCrate); addBox(4.2, 4.2, 2, PH + 1.1, 2, matCrate);
  addStrip(0, PH + .01, -5.95, 12, .02, .1); addStrip(0, PH + .01, 5.95, 12, .02, .1);
  for (let i = 0; i < 8; i++) {
    const a = i / 8 * Math.PI * 2 + Math.PI/8, x = Math.cos(a)*19, z = Math.sin(a)*19;
    addBox(x, z, 1.8, 10, 1.8, matPillar); addStrip(x, 3.4, z, 1.86, .12, 1.86);
  }
  const TH = STEP*10;
  for (const [sx, sz] of [[1,1],[1,-1],[-1,1],[-1,-1]]) {
    const cx = sx*38, cz = sz*38;
    addBox(cx, cz, 8, TH, 8, matConcrete);
    addBox(cx + sx*3.8, cz, .4, TH + 1, 8, matConcrete);
    addBox(cx, cz + sz*3.8, 8, TH + 1, .4, matConcrete);
    addStairs(cx - sx*4, cz - sz*1.5, -sx, 0, 3, TH, matConcrete);
    addStrip(cx - sx*3.95, TH + .01, cz, .1, .02, 8);
    addStrip(cx, TH + .01, cz - sz*3.95, 8, .02, .1);
  }
  for (const [x, z, ax, st, c] of [
    [44, -14, false, 1, 0], [47.5, -14, false, 2, 1], [44, -4, false, 1, 2], [51, -4, false, 1, 3],
    [44, 8, false, 2, 1], [47.5, 16, false, 1, 0], [51, 8, false, 1, 2],
    [-44, 14, false, 1, 3], [-47.5, 14, false, 2, 0], [-44, 4, false, 1, 1], [-51, 4, false, 1, 2],
    [-44, -8, false, 2, 3], [-47.5, -16, false, 1, 1], [-51, -8, false, 1, 0],
    [14, 46, true, 1, 2], [-6, 50, true, 2, 1], [4, 44, true, 1, 3],
    [-14, -46, true, 1, 0], [6, -50, true, 2, 2], [-4, -44, true, 1, 1]
  ]) addContainer(x, z, ax, st, c);
  for (const [x, z, w, h, d] of [
    [-10,-26,2,2,2],[10,26,2,2,2],[-26,10,2,2,2],[26,-10,2,2,2],
    [-28,-28,3,1.2,3],[28,28,3,1.2,3],[-28,28,2,2,2],[28,-28,2,2,2],
    [-16,32,4,1.2,2],[16,-32,4,1.2,2],[-30,-6,2,2,2],[30,6,2,2,2],
    [-12,-12,1,1.1,4],[12,12,1,1.1,4],[-12,12,4,1.1,1],[12,-12,4,1.1,1],
    [0,-28,6,1.1,.8],[0,28,6,1.1,.8],[-28,0,.8,1.1,6],[28,0,.8,1.1,6],
    [-22,-40,2,2,2],[22,40,2,2,2],[-40,-24,3,1.2,3],[40,24,3,1.2,3],
    [-34,0,2,1.1,4],[34,0,2,1.1,4],[0,-38,4,1.1,2],[0,38,4,1.1,2]
  ]) addBox(x, z, w, h, d, matCrate);
}

/* Fundición: arena compacta con piletas de metal fundido */
function buildFundicion() {
  addPerimeter(9, 7);
  addBox(0, 0, 6, 7, 6, matPillar);
  addVisual(0, 9, 0, 3, 4, 3, matPillar);
  for (const y of [2, 4.6]) addStrip(0, y, 0, 6.06, .14, 6.06);
  addVisual(0, 7.02, 0, 3.2, .04, 3.2, matLava, false);
  addLava(-15, 0, 6, 10); addLava(15, 0, 6, 10); addLava(0, -16, 10, 5); addLava(0, 16, 10, 5);
  for (const [x, z] of [[-9,-9],[9,-9],[-9,9],[9,9]]) { addBox(x, z, 1.6, 9, 1.6, matSteel); addStrip(x, 3, z, 1.66, .12, 1.66); }
  const PH = STEP*8;
  addBox(-26, -26, 10, PH, 10, matSteel); addStairs(-21, -28, 1, 0, 3, PH, matSteel);
  addBox(-26, -21.2, 10, PH + 1, .4, matSteel);
  addBox(26, 26, 10, PH, 10, matSteel); addStairs(21, 28, -1, 0, 3, PH, matSteel);
  addBox(26, 21.2, 10, PH + 1, .4, matSteel);
  addStrip(-21.05, PH + .01, -26, .1, .02, 10); addStrip(21.05, PH + .01, 26, .1, .02, 10);
  for (const [x, z] of [[-24,-12],[24,-12],[-24,12],[24,12]]) { addBox(x, z, 8, 1.1, 1.6, matSteel); addStrip(x, 1.12, z, 8, .02, .3); }
  for (const [x, z, w, h, d] of [
    [-6,22,2,2,2],[6,-22,2,2,2],[22,-6,2,2,2],[-22,6,2,2,2],[16,24,2,2,2],[-16,-24,2,2,2],
    [0,22,5,1.1,.8],[0,-22,5,1.1,.8],[26,-26,2.4,2.4,2.4],[-26,26,2.4,2.4,2.4],[23.5,-27,2,1.2,2],[-23.5,27,2,1.2,2]
  ]) addBox(x, z, w, h, d, matCrate);
}

/* Azotea: techo de un rascacielos de noche, abierto */
function buildAzotea() {
  addBox(0, -A, A*2+2, 1.3, 1.6, matConcrete); addBox(0, A, A*2+2, 1.3, 1.6, matConcrete);
  addBox(-A, 0, 1.6, 1.3, A*2+2, matConcrete); addBox(A, 0, 1.6, 1.3, A*2+2, matConcrete);
  addStrip(0, 1.32, -A+.6, A*2, .03, .2, matStripR); addStrip(0, 1.32, A-.6, A*2, .03, .2, matStripR);
  addStrip(-A+.6, 1.32, 0, .2, .03, A*2, matStripR); addStrip(A-.6, 1.32, 0, .2, .03, A*2, matStripR);
  const PH = STEP*3;
  addBox(0, 0, 14, PH, 14, matConcrete);
  addStairs(0, -7, 0, -1, 5, PH, matConcrete); addStairs(0, 7, 0, 1, 5, PH, matConcrete);
  addStrip(-2.2, PH + .01, 0, .5, .02, 5, matStripW); addStrip(2.2, PH + .01, 0, .5, .02, 5, matStripW); addStrip(0, PH + .01, 0, 4.4, .02, .5, matStripW);
  for (const [x, z, w, d] of [[0, -6.8, 14, .2], [0, 6.8, 14, .2], [-6.8, 0, .2, 14], [6.8, 0, .2, 14]]) addStrip(x, PH + .01, z, w, .02, d);
  addBox(-24, -20, 6, 4.2, 5, matConcrete); addStrip(-24, 1.2, -17.45, 1.6, 2.4, .1);
  addBox(24, 20, 5, 5, 5, matConcrete); addStrip(24, 1.2, 17.45, 1.6, 2.4, .1);
  for (const [x, z] of [[-32, 28], [-26, 32], [32, -28]]) {
    const t = new THREE.Mesh(tankGeo, matTank); t.position.set(x, 2.6, z); t.castShadow = t.receiveShadow = true; track(t);
    addVisual(x, .3, z, 3, .6, 3, matSteel);
    addCollider(x, z, 3, 4.6, 3);
  }
  addBox(34, 34, .6, 16, .6, matSteel); addStrip(34, 16.2, 34, .7, .4, .7, matStripR);
  for (const x of [-20, -14, 14, 20]) for (const z of [-30, 30]) addBox(x, z, 3, 1.6, 2, matPillar);
  for (let z = -12; z <= 12; z += 6) { addBox(-34, z, 5, .9, 2, matSteel); addBox(34, z, 5, .9, 2, matSteel); }
  for (const [x, z, w, h, d] of [[-12, 12, 2, 2, 2], [12, -12, 2, 2, 2], [-14, -10, 4, 1.1, 1], [14, 10, 4, 1.1, 1], [0, -24, 6, 1.1, .8], [0, 24, 6, 1.1, .8]]) addBox(x, z, w, h, d, matCrate);
  // skyline de fondo, sin colisión
  for (let i = 0; i < 48; i++) {
    const a = i / 48 * Math.PI * 2 + Math.random()*.08, r = A + 22 + Math.random()*70;
    const w = 10 + Math.random()*18, h = 30 + Math.random()*80, top = -24 + Math.random()*46;
    const m = new THREE.Mesh(boxGeo, matWindows[i % 3]);
    m.scale.set(w, h, w); m.position.set(Math.cos(a)*r, top - h/2, Math.sin(a)*r); m.rotation.y = -a;
    m.matrixAutoUpdate = false; m.updateMatrix(); track(m);
  }
}

const sandTex = canvasTex(256, (g, s) => {
  g.fillStyle = '#d6bb86'; g.fillRect(0, 0, s, s);
  for (let i = 0; i < 9000; i++) { const v = Math.random(); g.fillStyle = v < .5 ? 'rgba(150,110,60,.18)' : 'rgba(255,240,200,.22)'; g.fillRect(Math.random()*s, Math.random()*s, 2, 2); }
  g.strokeStyle = 'rgba(140,100,55,.18)'; g.lineWidth = 2;
  for (let y = 8; y < s; y += 18) { g.beginPath(); for (let x = 0; x <= s; x += 8) g.lineTo(x, y + Math.sin(x*.05 + y)*3); g.stroke(); }
});
const rockTex = canvasTex(256, (g, s) => {
  const bands = ['#b07a4a', '#c99460', '#9a6a40', '#d2a372', '#a87448', '#bd8a58'];
  let y = 0; while (y < s) { const h = 10 + Math.random()*28; g.fillStyle = bands[Math.random()*bands.length | 0]; g.fillRect(0, y, s, h); y += h; }
  for (let i = 0; i < 5000; i++) { g.fillStyle = `rgba(60,35,15,${Math.random()*.2})`; g.fillRect(Math.random()*s, Math.random()*s, 3, 2); }
  g.strokeStyle = 'rgba(70,40,20,.35)'; g.lineWidth = 2;
  for (let i = 0; i < 14; i++) { g.beginPath(); let x = Math.random()*s; g.moveTo(x, 0); for (let yy = 0; yy < s; yy += 20) { x += (Math.random()-.5)*14; g.lineTo(x, yy); } g.stroke(); }
});
const skyTex = canvasTex(256, (g, s) => {
  const gr = g.createLinearGradient(0, 0, 0, s);
  gr.addColorStop(0, '#3d7fc4'); gr.addColorStop(.42, '#8cc0e6'); gr.addColorStop(.5, '#dcecf2'); gr.addColorStop(.56, '#ead8b4'); gr.addColorStop(1, '#c9ac7c');
  g.fillStyle = gr; g.fillRect(0, 0, s, s);
  for (let i = 0; i < 18; i++) { const x = Math.random()*s, y = s*(.18 + Math.random()*.2), w = 30 + Math.random()*60; g.fillStyle = 'rgba(255,255,255,.35)'; g.beginPath(); g.ellipse(x, y, w, w*.18, 0, 0, Math.PI*2); g.fill(); }
});
const matRock = new THREE.MeshStandardMaterial({ map: rockTex, roughness: .95 });
const matYellow = new THREE.MeshStandardMaterial({ color: 0xe0a92a, roughness: .55, metalness: .3 });
const matTire = new THREE.MeshStandardMaterial({ color: 0x1a1a1a, roughness: .9 });
const matWater = new THREE.MeshStandardMaterial({ color: 0x3f8fae, roughness: .1, metalness: .3, transparent: true, opacity: .85 });
const matWood = new THREE.MeshStandardMaterial({ color: 0x8a6440, roughness: .85 });
const wheelGeo = new THREE.CylinderGeometry(.75, .75, .5, 16); wheelGeo.rotateZ(Math.PI/2);
function addTruck(x, z, alongX) {
  const w = alongX ? 7 : 3.2, d = alongX ? 3.2 : 7;
  addBox(x, z, w, 3, d, matYellow);
  const cab = alongX ? [x + 2.6, z, 1.8, 1.6, 3] : [x, z + 2.6, 3, 1.6, 1.8];
  addVisual(cab[0], 3.8, cab[1], cab[2], cab[3], cab[4], matYellow);
  addVisual(cab[0], 3.9, cab[1], cab[2] * .9, .7, cab[4] * .9, gm.glass, false);
  for (const a of [-1, 1]) for (const b of [-1, 1]) {
    const wm = new THREE.Mesh(wheelGeo, matTire); wm.castShadow = true;
    if (alongX) { wm.position.set(x + a*2.4, .75, z + b*1.75); } else { wm.rotation.y = Math.PI/2; wm.position.set(x + b*1.75, .75, z + a*2.4); }
    track(wm);
  }
}
function buildCantera() {
  // cielo y sol
  const sky = new THREE.Mesh(new THREE.SphereGeometry(190, 32, 16), new THREE.MeshBasicMaterial({ map: skyTex, side: THREE.BackSide, fog: false }));
  sky.material.map.repeat.set(1, 1); track(sky);
  const sunDisc = new THREE.Mesh(new THREE.SphereGeometry(7, 16, 12), new THREE.MeshBasicMaterial({ color: 0xfff4d6, fog: false }));
  sunDisc.position.copy(sun.position).normalize().multiplyScalar(180); track(sunDisc);
  // paredes de roca con escalones de piedra
  for (const [x, z, w, d] of [[0, -A, A*2+2, 2], [0, A, A*2+2, 2], [-A, 0, 2, A*2+2], [A, 0, 2, A*2+2]]) addBox(x, z, w, 14, d, matRock);
  for (let i = 0; i < 26; i++) {
    const side = i % 4, t = (Math.random()*2 - 1) * (A - 6), w = 3 + Math.random()*5, h = 3 + Math.random()*7, d = 2 + Math.random()*3;
    const [x, z] = side === 0 ? [t, -A + 1 + d/2] : side === 1 ? [t, A - 1 - d/2] : side === 2 ? [-A + 1 + d/2, t] : [A - 1 - d/2, t];
    if (Math.hypot(x - 0, z - 40) < 8) continue;
    addBox(x, z, side < 2 ? w : d, h, side < 2 ? d : w, matRock);
  }
  // meseta norte con dos escaleras
  const MH = STEP*8;
  addBox(0, -36, 36, MH, 14, matRock);
  addStairs(-10, -29, 0, 1, 4, MH, matRock); addStairs(10, -29, 0, 1, 4, MH, matRock);
  addBox(-14, -32, 4, MH + 1.1, 1, matWood); addBox(14, -32, 4, MH + 1.1, 1, matWood); addBox(0, -31, 6, MH + 1.1, 1, matWood);
  // terraza sudeste
  const TH = STEP*5;
  addBox(34, 30, 16, TH, 16, matRock); addStairs(26, 30, -1, 0, 4, TH, matRock);
  addBox(37, 27, 2, TH + 2, 2, matCrate);
  // agujas de roca
  for (const [x, z, w, h] of [[-22, -8, 3, 8], [22, -12, 3.5, 10], [-8, 14, 2.6, 7], [14, 10, 3, 9], [-30, -24, 4, 6], [30, 2, 2.6, 8], [-18, 30, 3, 7], [4, -14, 2.4, 6]]) {
    addBox(x, z, w, h, w, matRock); addBox(x + w*.4, z - w*.3, w*.7, h*.55, w*.8, matRock);
  }
  // camiones mineros
  addTruck(-14, 2, true); addTruck(18, 22, false); addTruck(-34, 4, false);
  // laguna (decorativa)
  const water = new THREE.Mesh(new THREE.PlaneGeometry(12, 9), matWater); water.rotation.x = -Math.PI/2; water.position.set(-28, .04, 24); track(water);
  for (const [x, z, w, d] of [[-28, 19, 13, .6], [-28, 29, 13, .6], [-34.8, 24, .6, 10.6]]) addBox(x, z, w, .5, d, matRock);
  // cajas y vigas de madera
  for (const [x, z, w, h, d] of [[0, 26, 6, 1.1, .8], [-6, 32, 2, 2, 2], [8, 34, 2, 1.2, 2], [24, -24, 4, 1.1, 1], [-24, -16, 2, 2, 2], [36, -12, 2, 2, 2], [-40, -36, 3, 1.2, 3], [6, 2, 4, 1.1, 1], [-4, -2, 1, 1.1, 4]]) addBox(x, z, w, h, d, matCrate);
  for (const [x, z] of [[-4, 40], [4, 40]]) addStrip(x, .03, z, .3, .02, 8, matWood);
}

const MAPS = {
  deposito: { name: 'Depósito', size: '120 m', desc: 'Complejo industrial con torres, contenedores y plaza elevada.', A: 60, start: [0, 24],
    bg: 0x0b1013, fog: [30, 120], hemi: [0x9fbccc, 0x2a2018, .75], sun: [0xffe2b8, 1.05], floor: 0xffffff, build: buildDeposito },
  fundicion: { name: 'Fundición', size: '68 m', desc: 'Arena cerrada y rápida. Las piletas de metal fundido queman.', A: 34, start: [0, 26],
    bg: 0x140b07, fog: [14, 66], hemi: [0xffb488, 0x2a1208, .62], sun: [0xffc89a, .8], floor: 0xc9a690, build: buildFundicion },
  cantera: { name: 'Cantera', size: '100 m', desc: 'Pleno sol en una cantera: meseta, agujas de roca y camiones mineros.', A: 50, start: [0, 40],
    bg: 0x9cc8e2, fog: [70, 200], hemi: [0xdcefff, 0xc8a878, 1.05], sun: [0xfff1d6, 1.7], floor: 0xffffff, floorTex: sandTex, build: buildCantera },
  azotea: { name: 'Azotea', size: '92 m', desc: 'Techo de un rascacielos de noche. Abierto y con poca cobertura.', A: 46, start: [0, 34],
    bg: 0x070b16, fog: [40, 190], hemi: [0x7f98d8, 0x101018, .62], sun: [0xa9c1ff, .75], floor: 0x8e9ab8, build: buildAzotea }
};
function buildMap(id) {
  for (const o of staticObjs) scene.remove(o);
  staticObjs = []; boxes.length = 0; grid.clear(); batches = new Map(); zones = [];
  MAP = MAPS[id]; mapId = id; A = MAP.A; LIM = A - 1.5;
  scene.background.setHex(MAP.bg); scene.fog.color.setHex(MAP.bg); scene.fog.far = Math.min(MAP.fog[1], Q.far - 10); scene.fog.near = Math.min(MAP.fog[0], scene.fog.far * .4);
  hemi.color.setHex(MAP.hemi[0]); hemi.groundColor.setHex(MAP.hemi[1]); hemi.intensity = MAP.hemi[2];
  sun.color.setHex(MAP.sun[0]); sun.intensity = MAP.sun[1];
  const sc = sun.shadow.camera; sc.left = sc.bottom = -A - 6; sc.right = sc.top = A + 6; sc.far = A*2 + 60; sc.updateProjectionMatrix();
  sun.position.set(A*.5, A*.85, A*.3);
  const ft = MAP.floorTex || floorTex; if (floorMat.map !== ft) { floorMat.map = ft; floorMat.needsUpdate = true; }
  floorMat.color.setHex(MAP.floor); ft.repeat.set(A/2, A/2);
  floor = new THREE.Mesh(new THREE.PlaneGeometry(A*2, A*2), floorMat);
  floor.rotation.x = -Math.PI/2; floor.receiveShadow = true; floor.matrixAutoUpdate = false; floor.updateMatrix(); track(floor);
  MAP.build();
  flushBatches();
  if (typeof targetsDirty !== 'undefined') targetsDirty = true;
  renderer.shadowMap.needsUpdate = true;
}

