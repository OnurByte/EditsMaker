// Sakura: pink cherry petals drifting down on a breeze, each tumbling as it falls; one lap per loop.
(THREE, scene, camera, opts) => {
  const count = 160, seconds = 8;
  let s = 23;
  const rand = () => ((s = (s * 16807) % 2147483647) / 2147483647);
  // a petal: two curves meeting in a notch, drawn once on a canvas
  const c = document.createElement("canvas");
  c.width = c.height = 128;
  const g = c.getContext("2d");
  const grad = g.createLinearGradient(0, 0, 0, 128);
  grad.addColorStop(0, "#ffe3ec");
  grad.addColorStop(1, "#f7a8c4");
  g.fillStyle = grad;
  g.beginPath();
  g.moveTo(64, 120);
  g.bezierCurveTo(8, 90, 12, 20, 50, 8);
  g.lineTo(64, 22);
  g.lineTo(78, 8);
  g.bezierCurveTo(116, 20, 120, 90, 64, 120);
  g.fill();
  const mat = new THREE.MeshBasicMaterial({ map: new THREE.CanvasTexture(c), transparent: true, side: THREE.DoubleSide, depthWrite: false });
  const geo = new THREE.PlaneGeometry(0.28, 0.28);
  const petals = [];
  for (let i = 0; i < count; i++) {
    const m = new THREE.Mesh(geo, mat);
    const p = { m, x: (rand() - 0.5) * 9, y: (rand() - 0.5) * 16, z: -1 - rand() * 8, seed: rand(), spin: 1 + Math.floor(rand() * 2) };
    scene.add(m);
    petals.push(p);
  }
  return (t) => {
    const k = (t % seconds) / seconds;
    for (const p of petals) {
      const w = (k + p.seed) * Math.PI * 2;
      p.m.position.set(p.x + Math.sin(w) * 0.6 + k * 2 - 1, ((p.y + 8 - k * 16) % 16 + 16) % 16 - 8, p.z);
      p.m.rotation.set(w * p.spin, w, Math.sin(w) * 0.8);
    }
  };
}
