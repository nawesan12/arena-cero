/* ---------- viewmodel: escena propia, se dibuja encima del mundo (no atraviesa paredes) ---------- */
const gunScene = new THREE.Scene();
const GUN_FOV = 60, GUN_FOV_AIM = 50;
const gunCam = new THREE.PerspectiveCamera(GUN_FOV, 1, .01, 10);
gunScene.add(gunCam);
gunScene.add(new THREE.HemisphereLight(0xcfe3ef, 0x2a2018, .55));
const gKey = new THREE.DirectionalLight(0xfff0dc, 1.25); gKey.position.set(-1, 2, 1.2); gunScene.add(gKey);
const gRim = new THREE.DirectionalLight(0x8fd8ff, .55); gRim.position.set(1.5, .4, -2); gunScene.add(gRim);
// mapa de entorno para reflejos metálicos: gradiente de depósito con tiras de luz
(function makeEnv() {
  const envScene = new THREE.Scene();
  const t = canvasTex(256, (g, s) => {
    const gr = g.createLinearGradient(0, 0, 0, s);
    gr.addColorStop(0, '#d9e4ea'); gr.addColorStop(.3, '#55636c'); gr.addColorStop(.55, '#20282d'); gr.addColorStop(1, '#0a0d0f');
    g.fillStyle = gr; g.fillRect(0, 0, s, s);
    g.fillStyle = '#ffd08a'; for (let x = 10; x < s; x += 64) g.fillRect(x, s*.18, 34, 5);
    g.fillStyle = '#ffb02e'; g.fillRect(0, s*.52, s, 2);
  });
  const sph = new THREE.Mesh(new THREE.SphereGeometry(5, 32, 16), new THREE.MeshBasicMaterial({ map: t, side: THREE.BackSide }));
  envScene.add(sph);
  const pm = new THREE.PMREMGenerator(renderer);
  gunScene.environment = pm.fromScene(envScene, .02).texture;
  pm.dispose();
})();

const gm = {
  metal: new THREE.MeshStandardMaterial({ color: 0x2f363c, metalness: .9, roughness: .3 }),
  dark: new THREE.MeshStandardMaterial({ color: 0x15181b, metalness: .7, roughness: .45 }),
  poly: new THREE.MeshStandardMaterial({ color: 0x1c2023, metalness: .05, roughness: .75 }),
  tan: new THREE.MeshStandardMaterial({ color: 0x5c4d39, metalness: .15, roughness: .65 }),
  brass: new THREE.MeshStandardMaterial({ color: 0xb8913a, metalness: 1, roughness: .3 }),
  shell: new THREE.MeshStandardMaterial({ color: 0x9a1e24, metalness: .2, roughness: .5 }),
  glass: new THREE.MeshStandardMaterial({ color: 0x9fe9ff, metalness: 1, roughness: 0, transparent: true, opacity: .16, depthWrite: false }),
  amber: new THREE.MeshBasicMaterial({ color: 0xffb02e }),
  red: new THREE.MeshBasicMaterial({ color: 0xff3d5a }),
  cyan: new THREE.MeshBasicMaterial({ color: 0x5fd8ff })
};
const cylGeo = new THREE.CylinderGeometry(1, 1, 1, 20); cylGeo.rotateX(Math.PI/2); // eje sobre Z
const tubeGeo = new THREE.CylinderGeometry(1, 1, 1, 24, 1, true); tubeGeo.rotateX(Math.PI/2); // tubo abierto para miras
function B(p, w, h, d, x, y, z, m, rx = 0) { const o = new THREE.Mesh(boxGeo, m); o.scale.set(w, h, d); o.position.set(x, y, z); o.rotation.x = rx; p.add(o); return o; }
function C(p, r, len, x, y, z, m) { const o = new THREE.Mesh(cylGeo, m); o.scale.set(r, r, len); o.position.set(x, y, z); p.add(o); return o; }
function T(p, r, tube, x, y, z, m) { const o = new THREE.Mesh(new THREE.TorusGeometry(r, tube, 8, 24), m); o.position.set(x, y, z); p.add(o); return o; }

