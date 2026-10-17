/* ---------- HUD (solo escribe al DOM cuando algo cambia) ---------- */
const hud = { wave: $('wave'), left: $('left'), score: $('score'), kills: $('kills'), combo: $('combo'), hpnum: $('hpnum'), railscope: $('railscope'), lvl: $('lvl'), hpfill: $('hpfill'), dashfill: $('dashfill'), boss: $('boss'), bossfill: $('bossfill'), stance: $('stance'),
  ammo: $('ammo'), reload: $('reload'), wname: $('wname'), slots: $('slots'), nades: $('nades'), vig: $('vig'), hit: $('hit'), dmgdir: $('dmgdir'),
  banner: $('banner'), cross: $('cross'), scope: $('scope'), fps: $('fps'), wlabel: $('wlabel'), lives: $('lives'), alertpill: $('alertpill'), prompt: $('prompt'), ptext: $('ptext'), pfill: $('pfill') };
const last = {};
function setTxt(k, v) { if (last[k] !== v) { last[k] = v; hud[k].textContent = v; } }
function setStyle(k, prop, v) { const key = k + prop; if (last[key] !== v) { last[key] = v; hud[k].style[prop] = v; } }
function updateHud(dt) {
  const W = WEAPONS[P.w];
  if (isCity()) {
    setTxt('wlabel', 'Objetivos');
    setTxt('wave', `${cityState.done.length}/${OUTPOSTS.length}`);
    const m = Math.floor(cityState.time / 60), s = Math.floor(cityState.time % 60);
    setTxt('left', `${MAP.name} · ${m}:${String(s).padStart(2, '0')}`);
    setTxt('lvl', !cityState.powered || !cityState.antenna || cityState.intel ? [!cityState.antenna && 'sin refuerzos', !cityState.powered && 'torretas apagadas', cityState.intel && 'enemigos marcados'].filter(Boolean).join(' · ') : '');
    setTxt('lives', 'INTENTOS ' + '◆'.repeat(cityState.lives) + '◇'.repeat(Math.max(0, 3 - cityState.lives)));
    const al = cityAlert();
    setTxt('alertpill', ['OCULTO', 'BUSCANDO', 'DETECTADO'][al]);
    if (last.al !== al) { last.al = al; hud.alertpill.className = 'hud a' + al; }
    const show = !!interact;
    if (last.pr !== show) { last.pr = show; hud.prompt.hidden = !show; }
    if (interact) { setTxt('ptext', interact.label); setStyle('pfill', 'width', interact.time ? `${Math.min(100, holdT / interact.time * 100).toFixed(0)}%` : '0%'); }
  } else {
    setTxt('wlabel', 'Oleada');
    setTxt('wave', String(wave).padStart(2, '0'));
    const left = enemies.length + spawnQueue.length;
    setTxt('lvl', MAP ? (gameMode === 'campaign' ? `Nivel ${levelIdx + 1} · ${MAP.name}` : MAP.name) : '');
    setTxt('left', wave ? `${left} hostil${left === 1 ? '' : 'es'}` : 'Preparate');
    if (last.pr) { last.pr = false; hud.prompt.hidden = true; }
  }
  setTxt('score', score.toLocaleString('es-AR'));
  setTxt('kills', `${kills} baja${kills === 1 ? '' : 's'}`);
  setTxt('combo', combo > 1 ? `COMBO ×${combo}` : '');
  setTxt('hpnum', String(Math.ceil(P.hp)));
  const hr = P.hp / P.maxHp;
  setStyle('hpfill', 'transform', `scaleX(${hr.toFixed(3)})`);
  setStyle('hpfill', 'background', hr > .5 ? 'var(--mint)' : hr > .25 ? 'var(--amber)' : 'var(--red)');
  setStyle('dashfill', 'transform', `scaleX(${(1 - Math.max(0, P.dashCd) / (3 * P.dashCdMul)).toFixed(2)})`);
  setStyle('dashfill', 'opacity', P.dashCd <= 0 ? '1' : '.45');
  const showBoss = !!bossRef;
  if (last.boss !== showBoss) { last.boss = showBoss; hud.boss.hidden = !showBoss; }
  if (bossRef) setStyle('bossfill', 'transform', `scaleX(${Math.max(0, bossRef.hp / bossRef.maxHp).toFixed(3)})`);
  setTxt('stance', P.overdrive > 0 ? `SOBRECARGA ${Math.ceil(P.overdrive)} s` : P.dashT > 0 ? 'IMPULSO' : P.slide > 0 ? 'DESLIZANDO' : P.crouchT > .5 ? 'AGACHADO' : '');
  const mg = magOf(P.w), am = `${P.w}|${P.ammo[P.w]}|${mg}`;
  if (last.am !== am) {
    last.am = am; hud.ammo.innerHTML = `${P.ammo[P.w]}<small> / ${mg}</small>`;
    hud.ammo.style.color = P.ammo[P.w] <= Math.ceil(magOf(P.w)/4) ? 'var(--red)' : '';
  }
  setTxt('wname', W.name);
  const sk = P.w + '|' + (P.unlocked ? P.unlocked.join('') : 'all');
  if (last.slotsKey !== sk) { last.slotsKey = sk; for (const s of hud.slots.children) { s.classList.toggle('on', +s.dataset.w === P.w); s.classList.toggle('locked', !!P.unlocked && !P.unlocked[+s.dataset.w]); } }
  const nk = `${P.nades}/${P.maxNades}`;
  if (last.nd !== nk) { last.nd = nk; hud.nades.innerHTML = '<span class="label">G</span>' + Array.from({ length: P.maxNades }, (_, i) => `<i class="${i < P.nades ? 'full' : ''}"></i>`).join(''); }
  setTxt('reload', P.reload > 0 ? 'RECARGANDO' : P.ammo[P.w] <= Math.ceil(magOf(P.w)/4) && P.ammo[P.w] > 0 ? 'R PARA RECARGAR' : '');
  hurtFlash = Math.max(0, hurtFlash - dt*1.6);
  const lowHp = P.hp < 30 ? .35 + Math.sin(performance.now()/180)*.1 : 0;
  setStyle('vig', 'opacity', Math.max(hurtFlash, lowHp).toFixed(2));
  hitT -= dt; setStyle('hit', 'opacity', hitT > 0 ? '1' : '0');
  dmgT = Math.max(0, dmgT - dt * 1.2); setStyle('dmgdir', 'opacity', dmgT.toFixed(2));
  const aimOn = P.aimT > .5;
  if (last.aimOn !== aimOn) { last.aimOn = aimOn; hud.cross.classList.toggle('aim', aimOn); }
  // la cruz se abre con el movimiento y el arma
  const moving = Math.hypot(P.vel.x, P.vel.z) > 1 || !P.onGround;
  const gap = Math.round(((moving ? W.spread[1] : W.spread[0]) * 600 + P.recoil * 6) * (1 - P.crouchT * .4));
  setStyleVar(gap + 'px');
  const rs = P.w === 2 ? Math.max(0, Math.min(1, (P.aimT - .6) / .3)) : 0;
  setStyle('railscope', 'opacity', rs.toFixed(2));
  setStyle('scope', 'opacity', (P.aimT * (1 - rs)).toFixed(2));
  if (bannerT > 0) { bannerT -= dt; if (bannerT <= 0) hud.banner.style.opacity = 0; }
}
function setStyleVar(v) { if (last.g !== v) { last.g = v; hud.cross.style.setProperty('--g', v); } }


