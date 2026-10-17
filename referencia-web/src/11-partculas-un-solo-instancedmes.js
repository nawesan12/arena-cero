/* ---------- partículas: un solo InstancedMesh ---------- */
const MAXP = 600;
const pMesh = new THREE.InstancedMesh(new THREE.BoxGeometry(.09, .09, .09), new THREE.MeshBasicMaterial({ color: 0xffffff }), MAXP);
pMesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
pMesh.frustumCulled = false; pMesh.count = 0;
const _col = new THREE.Color();
for (let i = 0; i < MAXP; i++) pMesh.setColorAt(i, _col);
scene.add(pMesh);
const pPos = new Float32Array(MAXP*3), pVel = new Float32Array(MAXP*3), pLife = new Float32Array(MAXP), pMax = new Float32Array(MAXP), pCol = new Uint32Array(MAXP);
let pN = 0;
function burst(pos, color, n, speed, life = .5) {
  for (let k = 0; k < n && pN < MAXP; k++) {
    const i = pN++;
    let vx = Math.random()-.5, vy = Math.random()-.2, vz = Math.random()-.5;
    const l = Math.hypot(vx, vy, vz) || 1, s = speed * (.4 + Math.random()*.8) / l;
    pPos[i*3] = pos.x; pPos[i*3+1] = pos.y; pPos[i*3+2] = pos.z;
    pVel[i*3] = vx*s; pVel[i*3+1] = vy*s; pVel[i*3+2] = vz*s;
    pLife[i] = pMax[i] = life * (.6 + Math.random()*.6);
    pCol[i] = color; pMesh.setColorAt(i, _col.setHex(color));
  }
  pMesh.instanceColor.needsUpdate = true;
}
const _m4 = new THREE.Matrix4();
function updateParticles(dt) {
  let colorDirty = false;
  for (let i = 0; i < pN; i++) {
    pLife[i] -= dt;
    if (pLife[i] <= 0) {
      const j = --pN;
      if (i !== j) {
        pPos[i*3] = pPos[j*3]; pPos[i*3+1] = pPos[j*3+1]; pPos[i*3+2] = pPos[j*3+2];
        pVel[i*3] = pVel[j*3]; pVel[i*3+1] = pVel[j*3+1]; pVel[i*3+2] = pVel[j*3+2];
        pLife[i] = pLife[j]; pMax[i] = pMax[j]; pCol[i] = pCol[j];
        pMesh.setColorAt(i, _col.setHex(pCol[i])); colorDirty = true;
      }
      i--; continue;
    }
    pVel[i*3+1] -= 14*dt;
    pPos[i*3] += pVel[i*3]*dt; pPos[i*3+1] += pVel[i*3+1]*dt; pPos[i*3+2] += pVel[i*3+2]*dt;
    if (pPos[i*3+1] < .05) { pPos[i*3+1] = .05; pVel[i*3+1] *= -.3; pVel[i*3] *= .6; pVel[i*3+2] *= .6; }
    const s = Math.max(.01, pLife[i] / pMax[i]);
    _m4.makeScale(s, s, s).setPosition(pPos[i*3], pPos[i*3+1], pPos[i*3+2]);
    pMesh.setMatrixAt(i, _m4);
  }
  pMesh.count = pN;
  pMesh.instanceMatrix.needsUpdate = true;
  if (colorDirty) pMesh.instanceColor.needsUpdate = true;
}

