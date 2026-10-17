/* ---------- renderer / escena ---------- */
const renderer = new THREE.WebGLRenderer({ antialias: !isTouch, powerPreference: 'high-performance' });
// calidad: limita FPS, resolución, sombras y distancia de dibujado (menos calor en notebooks)
const QUALITY = { baja: { pr: .75, shadow: 1024, far: 200, fps: 40 }, media: { pr: 1, shadow: 2048, far: 280, fps: 60 }, alta: { pr: 1.6, shadow: 4096, far: 360, fps: 0 } };
let quality = 'media'; try { quality = localStorage.getItem('arena-cero-q') || (isTouch ? 'baja' : 'media'); } catch (e) {}
let Q = QUALITY[quality] || QUALITY.media;
let MAX_PR = Math.min(devicePixelRatio, Q.pr);
let pixelRatio = MAX_PR;
renderer.setPixelRatio(pixelRatio);
renderer.outputEncoding = THREE.sRGBEncoding;
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
renderer.shadowMap.autoUpdate = false; // geometría estática: sombra se calcula una sola vez
renderer.autoClear = false; // mundo + viewmodel en dos pasadas
$('stage').appendChild(renderer.domElement);

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x0b1013);
scene.fog = new THREE.Fog(0x0b1013, 30, 120);
const BASE_FOV = 75, AIM_FOV = 48;
const camera = new THREE.PerspectiveCamera(BASE_FOV, 1, 0.05, Q.far);
camera.rotation.order = 'YXZ';
scene.add(camera);

const hemi = new THREE.HemisphereLight(0x9fbccc, 0x2a2018, 0.75); scene.add(hemi);
const sun = new THREE.DirectionalLight(0xffe2b8, 1.05);
sun.position.set(30, 50, 18);
sun.castShadow = true;
sun.shadow.mapSize.set(Q.shadow, Q.shadow);
Object.assign(sun.shadow.camera, { left: -66, right: 66, top: 66, bottom: -66, near: 1, far: 120 });
scene.add(sun);
// luz de explosiones: siempre presente (intensidad 0) para no recompilar shaders
const boomLight = new THREE.PointLight(0xffa040, 0, 18, 2); scene.add(boomLight);

