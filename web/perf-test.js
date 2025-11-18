// Performance testing for NEARx DOM implementation
(function() {
  const measurements = [];

  // Hook into render function
  const originalRender = window.render;
  if (!originalRender) {
    console.error('Render function not found');
    return;
  }

  window.render = function(snapshot) {
    const start = performance.now();
    originalRender.call(this, snapshot);
    const end = performance.now();
    const duration = end - start;

    measurements.push(duration);

    // Log every 10th measurement
    if (measurements.length % 10 === 0) {
      const avg = measurements.slice(-10).reduce((a, b) => a + b) / 10;
      console.log(`Render performance: last 10 avg = ${avg.toFixed(2)}ms`);
    }

    // Show in UI
    const perfEl = document.getElementById('nearx-perf') || createPerfElement();
    perfEl.textContent = `Render: ${duration.toFixed(1)}ms (avg: ${avg.toFixed(1)}ms)`;
  };

  function createPerfElement() {
    const el = document.createElement('div');
    el.id = 'nearx-perf';
    el.style.cssText = `
      position: fixed;
      top: 5px;
      right: 5px;
      background: rgba(0, 0, 0, 0.8);
      color: #00ff00;
      padding: 5px 10px;
      font-family: monospace;
      font-size: 12px;
      border: 1px solid #00ff00;
      z-index: 10000;
    `;
    document.body.appendChild(el);
    return el;
  }

  console.log('Performance monitoring enabled');
})();