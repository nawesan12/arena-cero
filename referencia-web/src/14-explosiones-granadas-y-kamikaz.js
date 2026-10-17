/* ---------- explosiones (granadas y kamikazes) ---------- */
let boomT = 0;
function explode(pos, radius, maxDmg, selfDmg, big = true) {
  if (isCity()) makeNoise(pos, big ? 80 : 45);
  burst(pos, 0xffb02e, big ? 60 : 30, big ? 12 : 8, .7);
  burst(pos, 0xff3d5a, big ? 24 : 12, 9, .6);
  burst(pos, 0x2a2a2a, big ? 20 : 8, 5, 1.2);
  boomLight.position.copy(pos); boomLight.intensity = big ? 6 : 3; boomT = .25;
  if (big) sfx.boom();
  for (let i = enemies.length - 1; i >= 0; i--) {
    const e = enemies[i], d = e.g.position.distanceTo(pos);
    if (d < radius + e.radius) {
      const dmg = Math.max(1, Math.round(maxDmg * (1 - d / (radius + e.radius))));
      _c.copy(e.g.position).sub(pos).normalize();
      damageEnemy(e, dmg, _c);
    }
  }
  const pd = _c.set(P.pos.x, P.pos.y - .8, P.pos.z).distanceTo(pos);
  P.shake = Math.min(1, P.shake + (big ? .9 : .4) * Math.max(0, 1 - pd / 20));
  if (selfDmg && pd < radius) hurt(Math.round(selfDmg * (1 - pd / radius)), pos);
}

