// Stardust: soft points of light drifting towards you, turning slowly; the last frame meets the first, so it loops.
(THREE, scene, camera, opts) => {
  const count = 220, seconds = 6;
  const pos = new Float32Array(count * 3), seed = new Float32Array(count);
  let s = 7;
  const rand = () => ((s = (s * 16807) % 2147483647) / 2147483647);
  for (let i = 0; i < count; i++) {
    pos[i * 3] = (rand() - 0.5) * 9;
    pos[i * 3 + 1] = (rand() - 0.5) * 16;
    pos[i * 3 + 2] = rand() * -12;
    seed[i] = rand();
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.BufferAttribute(pos.slice(), 3));
  // each point its own warmth and brightness, a few large and soft like lights out of focus
  const col = new Float32Array(count * 3);
  for (let i = 0; i < count; i++) {
    const b = 0.55 + 0.45 * seed[i];
    col[i * 3] = b; col[i * 3 + 1] = b * 0.92; col[i * 3 + 2] = b * 0.8;
  }
  geo.setAttribute("color", new THREE.BufferAttribute(col, 3));
  // a round soft dot, drawn once
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const g = c.getContext("2d");
  const grad = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  grad.addColorStop(0, "rgba(255,255,255,1)");
  grad.addColorStop(0.25, "rgba(255,236,214,0.75)");
  grad.addColorStop(1, "rgba(255,236,214,0)");
  g.fillStyle = grad;
  g.fillRect(0, 0, 64, 64);
  const mat = new THREE.PointsMaterial({ size: 0.5, vertexColors: true, map: new THREE.CanvasTexture(c), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending });
  const points = new THREE.Points(geo, mat);
  scene.add(points);
  return (t) => {
    const k = (t % seconds) / seconds;
    const a = geo.attributes.position.array;
    for (let i = 0; i < count; i++) {
      // each point travels one full lap of depth per loop, so frame 0 and the last match
      a[i * 3 + 2] = ((pos[i * 3 + 2] + 12 + k * 12) % 12) - 12;
      a[i * 3 + 1] = pos[i * 3 + 1] + Math.sin((k + seed[i]) * Math.PI * 2) * 0.15;
    }
    geo.attributes.position.needsUpdate = true;
    points.rotation.z = Math.sin(k * Math.PI * 2) * 0.12;
    mat.opacity = 1;
  };
}
