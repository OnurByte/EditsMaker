// Snow, after three.js's webgl_points_sprites example (MIT): soft flakes of three sizes falling and swaying; each
// falls one full height per loop, so the last frame meets the first.
(THREE, scene, camera, opts) => {
  const seconds = 8, layers = [[500, 0.12], [350, 0.2], [150, 0.32]];
  let s = 11;
  const rand = () => ((s = (s * 16807) % 2147483647) / 2147483647);
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const g = c.getContext("2d");
  const grad = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  grad.addColorStop(0, "rgba(255,255,255,1)");
  grad.addColorStop(0.5, "rgba(240,246,255,0.6)");
  grad.addColorStop(1, "rgba(240,246,255,0)");
  g.fillStyle = grad;
  g.fillRect(0, 0, 64, 64);
  const tex = new THREE.CanvasTexture(c);
  const sets = layers.map(([count, size], li) => {
    const base = new Float32Array(count * 3), seed = new Float32Array(count);
    for (let i = 0; i < count; i++) {
      base[i * 3] = (rand() - 0.5) * 10;
      base[i * 3 + 1] = (rand() - 0.5) * 16;
      base[i * 3 + 2] = -1 - rand() * 10;
      seed[i] = rand();
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(base.slice(), 3));
    const mat = new THREE.PointsMaterial({ size, map: tex, transparent: true, depthWrite: false, opacity: 0.55 + li * 0.15 });
    scene.add(new THREE.Points(geo, mat));
    return { geo, base, seed, count, laps: li + 1 };
  });
  return (t) => {
    const k = (t % seconds) / seconds;
    for (const { geo, base, seed, count, laps } of sets) {
      const a = geo.attributes.position.array;
      for (let i = 0; i < count; i++) {
        // nearer flakes fall faster: a whole number of laps per loop
        a[i * 3 + 1] = ((base[i * 3 + 1] + 8 - k * 16 * laps) % 16 + 16) % 16 - 8;
        a[i * 3] = base[i * 3] + Math.sin((k * laps + seed[i]) * Math.PI * 2) * 0.25;
      }
      geo.attributes.position.needsUpdate = true;
    }
  };
}
