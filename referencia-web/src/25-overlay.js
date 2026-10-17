/* ---------- overlay ---------- */
function showOverlay(mode, isBest) {
  const ov = $('overlay'), st = $('stats'), go = $('go'), intro = $('intro');
  ov.hidden = false;
  $('mapsect').hidden = mode === 'pause';
  showBest();
  if ((mode === 'over' || mode === 'win') && isCity()) {
    const m = Math.floor(cityState.time / 60), s = Math.floor(cityState.time % 60);
    intro.textContent = mode === 'win' ? (isBest ? 'Distrito liberado y nuevo récord.' : 'Distrito liberado. Probá hacerlo más rápido o sin que te vean.') : 'Te quedaste sin intentos. Probá otra entrada: el sigilo y la antena cambian todo.';
    st.hidden = false;
    st.innerHTML = `<div><span class="label">Puntaje</span><span class="v mono">${score.toLocaleString('es-AR')}</span></div><div><span class="label">Objetivos</span><span class="v mono">${cityState.done.length}/${OUTPOSTS.length}</span></div><div><span class="label">Bajas</span><span class="v mono">${kills}</span></div><div><span class="label">Tiempo</span><span class="v mono">${m}:${String(s).padStart(2, '0')}</span></div>`;
    go.textContent = 'Jugar otra vez';
  } else if (mode === 'over') {
    intro.textContent = isBest ? 'Nuevo récord. El depósito te ganó igual.' : 'Te desmantelaron. Probá otra vez.';
    st.hidden = false;
    st.innerHTML = `<div><span class="label">Puntaje</span><span class="v mono">${score.toLocaleString('es-AR')}</span></div><div><span class="label">Oleada</span><span class="v mono">${wave}</span></div><div><span class="label">Bajas</span><span class="v mono">${kills}</span></div>`;
    go.textContent = 'Reintentar';
  } else if (mode === 'pause') {
    intro.textContent = 'Juego en pausa.'; st.hidden = true; go.textContent = 'Continuar';
  }
  ov.dataset.mode = mode;
}
function begin() {
  initAudio(); startMusic();
  const mode = $('overlay').dataset.mode || 'menu';
  if (mode !== 'pause') {
    if (gameMode === 'city') { if (mapId !== 'ciudad' || cityDirty) buildMap('ciudad'); cityDirty = true; }
    else if (gameMode === 'campaign' && mapId !== CAMPAIGN[0]) buildMap(CAMPAIGN[0]);
    resetGame();
  }
  state = 'playing'; $('overlay').hidden = true; radarEl.hidden = false;
  clock.getDelta();
  if (!isTouch && renderer.domElement.requestPointerLock) {
    try { const r = renderer.domElement.requestPointerLock(); if (r && r.catch) r.catch(() => {}); } catch (e) {}
  }
}
$('go').addEventListener('click', begin);
$('cards').addEventListener('click', e => { const b = e.target.closest('button[data-i]'); if (b) pickPerk(+b.dataset.i); });
showBest();
$('vol').addEventListener('input', e => setMusicVol(+e.target.value));
setMusicVol(musicVol);
const sensIn = $('sens'), sensOut = $('sensv');
sensIn.value = sens; sensOut.textContent = sens.toFixed(1);
sensIn.addEventListener('input', () => { sens = +sensIn.value; sensOut.textContent = sens.toFixed(1); try { localStorage.setItem('arena-cero-sens', sens); } catch (e) {} });

