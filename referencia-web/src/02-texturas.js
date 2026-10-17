/* ---------- texturas ---------- */
function canvasTex(size, draw, rep) {
  const c = document.createElement('canvas'); c.width = c.height = size;
  draw(c.getContext('2d'), size);
  const t = new THREE.CanvasTexture(c);
  t.wrapS = t.wrapT = THREE.RepeatWrapping;
  if (rep) t.repeat.set(rep, rep);
  t.encoding = THREE.sRGBEncoding;
  t.anisotropy = Math.min(4, renderer.capabilities.getMaxAnisotropy());
  return t;
}
const floorTex = canvasTex(512, (g, s) => {
  g.fillStyle = '#2b3134'; g.fillRect(0, 0, s, s);
  for (let i = 0; i < 9000; i++) { const v = 30 + Math.random() * 30 | 0; g.fillStyle = `rgba(${v},${v+4},${v+6},.35)`; g.fillRect(Math.random()*s, Math.random()*s, 2, 2); }
  g.strokeStyle = 'rgba(0,0,0,.55)'; g.lineWidth = 3; g.strokeRect(0, 0, s, s);
  g.strokeStyle = 'rgba(255,176,46,.10)'; g.lineWidth = 1; g.beginPath(); g.moveTo(s/2,0); g.lineTo(s/2,s); g.moveTo(0,s/2); g.lineTo(s,s/2); g.stroke();
}, 16);
const crateTex = canvasTex(256, (g, s) => {
  g.fillStyle = '#463b2c'; g.fillRect(0, 0, s, s);
  for (let i = 0; i < 2500; i++) { g.fillStyle = `rgba(0,0,0,${Math.random()*.25})`; g.fillRect(Math.random()*s, Math.random()*s, 3, 1); }
  g.strokeStyle = '#2a2219'; g.lineWidth = 14; g.strokeRect(7, 7, s-14, s-14);
  g.save(); g.beginPath(); g.rect(14, s-46, s-28, 32); g.clip();
  for (let x = -40; x < s; x += 28) { g.fillStyle = '#d99a2b'; g.beginPath(); g.moveTo(x, s-14); g.lineTo(x+14, s-14); g.lineTo(x+46, s-46); g.lineTo(x+32, s-46); g.fill(); }
  g.restore();
});
const glowTex = canvasTex(64, (g, s) => {
  const gr = g.createRadialGradient(s/2, s/2, 0, s/2, s/2, s/2);
  gr.addColorStop(0, 'rgba(255,240,200,1)'); gr.addColorStop(.3, 'rgba(255,170,50,.8)'); gr.addColorStop(1, 'rgba(255,120,0,0)');
  g.fillStyle = gr; g.fillRect(0, 0, s, s);
});

