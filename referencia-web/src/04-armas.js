/* ---------- armas ---------- */
const WEAPONS = [
  { name: 'Rifle de pulso', mag: 24, rate: .1, reload: 1.3, dmg: 1, pellets: 1, spread: [.004, .012], kick: .009, auto: true, color: 0xffc66a, aim: [-.114, -.27], muzzle: [.004, -.6] },
  { name: 'Escopeta de dispersión', mag: 6, rate: .7, reload: 1.7, dmg: 1, pellets: 9, spread: [.075, .095], kick: .045, auto: false, color: 0xffc66a, aim: [-.08, -.25], muzzle: [.02, -.6] },
  { name: 'Riel de partículas', mag: 4, rate: 1.05, reload: 1.9, dmg: 6, pellets: 1, spread: [0, .006], kick: .03, auto: false, pierce: true, color: 0x5fd8ff, aim: [-.096, -.2], muzzle: [0, -.66] },
  { name: 'Lanzagranadas', mag: 3, rate: .75, reload: 2.2, dmg: 0, pellets: 0, spread: [.004, .02], kick: .05, auto: false, launcher: true, color: 0xffb02e, aim: [-.1, -.26], muzzle: [.01, -.56] }
];

