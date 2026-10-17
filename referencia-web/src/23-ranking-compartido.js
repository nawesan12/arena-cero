/* ---------- ranking compartido ---------- */
const LB = { db: null, user: null, me: null, canWrite: null, unsub: null, rows: [], gen: 0 };
(async () => {
  const cl = window.claude; if (!cl || !cl.use) return;
  const [db, user] = await Promise.all([cl.use('db'), cl.use('user')]);
  if (!db) return;
  LB.db = db; LB.user = user;
  LB.me = user ? await user.id() : null;
  LB.canWrite = user ? await user.can('data.write') : null;
  $('lb').hidden = false;
  subscribeLB();
})();
function subscribeLB() {
  if (!LB.db) return;
  if (LB.unsub) { LB.unsub(); LB.unsub = null; }
  LB.rows = []; renderLB();
  LB.unsub = LB.db.collection('lb_' + lbKey()).orderBy('score', 'desc').limit(10).onSnapshot(
    snap => { LB.rows = snap.docs.map(d => ({ id: d.id, ...d.data() })); renderLB(); },
    () => { $('lbnote').textContent = 'No se pudo cargar el ranking.'; });
}
async function renderLB() {
  const gen = ++LB.gen, rows = LB.rows.slice();
  const ps = LB.user && rows.length ? await LB.user.profiles(rows.map(r => r.id)) : {};
  if (gen !== LB.gen) return;
  const list = $('lblist'); list.textContent = '';
  if (!rows.length) { const li = document.createElement('li'); li.className = 'empty'; li.textContent = 'Todavía no hay puntajes en este mapa. El primero queda arriba.'; list.appendChild(li); }
  rows.forEach((r, i) => {
    const li = document.createElement('li'); if (r.id === LB.me) li.className = 'me';
    const p = ps[r.id] || {};
    const cells = [['r', String(i + 1).padStart(2, '0')], ['nm', r.id === LB.me ? `${p.name || 'Vos'} (vos)` : (p.name || 'Jugador')], ['w', `OL. ${r.wave || 0}`], ['s', (r.score || 0).toLocaleString('es-AR')]];
    for (const [c, t] of cells) { const sp = document.createElement('span'); sp.className = c; sp.textContent = t; li.appendChild(sp); }
    list.appendChild(li);
  });
  $('lbnote').textContent = LB.canWrite === false ? 'Tenés acceso de solo lectura: tus partidas no entran al ranking.' : !LB.me ? 'Iniciá sesión para aparecer en el ranking.' : 'Se guarda tu mejor puntaje en cada mapa.';
}
async function submitScore() {
  if (!LB.db || !LB.me || LB.canWrite === false || score <= 0) return;
  const ref = LB.db.doc(`lb_${lbKey()}/${LB.me}`);
  try {
    const cur = await ref.get();
    const prev = cur.exists ? (cur.data().score || 0) : 0;
    if (score > prev) await ref.set({ score, wave, kills, at: Date.now() });
  } catch (e) { if (e && e.code === 'invalid_argument') { LB.canWrite = false; renderLB(); } }
}

