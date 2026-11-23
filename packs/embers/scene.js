// Embers: sparks rising out of a fire below, flickering orange to red and fading as they climb; one lap per loop.
(THREE, scene, camera, opts) => {
  const count = 700, seconds = 5;
  let s = 5;
  const rand = () => ((s = (s * 16807) % 2147483647) / 2147483647);
  const base = new Float32Array(count * 3), seed = new Float32Array(count), col = new Float32Array(count * 3);
  for (let i = 0; i < count; i++) {
    base[i * 3] = (rand() - 0.5) * 9;
    base[i * 3 + 1] = rand();
    base[i * 3 + 2] = -1 - rand() * 8;
    seed[i] = rand();
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.BufferAttribute(new Float32Array(count * 3), 3));
  geo.setAttribute("color", new THREE.BufferAttribute(col, 3));
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const g = c.getContext("2d");
  const grad = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  grad.addColorStop(0, "rgba(255,255,255,1)");
  grad.addColorStop(0.25, "rgba(255,255,255,0.7)");
  grad.addColorStop(1, "rgba(255,255,255,0)");
  g.fillStyle = grad;
  g.fillRect(0, 0, 64, 64);
  const mat = new THREE.PointsMaterial({ size: 0.16, map: new THREE.CanvasTexture(c), vertexColors: true, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending });
  scene.add(new THREE.Points(geo, mat));
  return (t) => {
    const k = (t % seconds) / seconds;
    const a = geo.attributes.position.array;
    for (let i = 0; i < count; i++) {
      // how far up this spark is, 0 at the fire to 1 at the top
      const h = (base[i * 3 + 1] + k) % 1;
      const w = (k * 2 + seed[i]) * Math.PI * 2;
      a[i * 3] = base[i * 3] + Math.sin(w) * 0.3 * h;
      a[i * 3 + 1] = -8 + h * 16;
      a[i * 3 + 2] = base[i * 3 + 2];
      const fade = (1 - h) * (0.7 + 0.3 * Math.sin(w * 3));
      col[i * 3] = fade;
      col[i * 3 + 1] = fade * (0.55 - 0.4 * h);
      col[i * 3 + 2] = fade * 0.1;
    }
    geo.attributes.position.needsUpdate = true;
    geo.attributes.color.needsUpdate = true;
  };
}
