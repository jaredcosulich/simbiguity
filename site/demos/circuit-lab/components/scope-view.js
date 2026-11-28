export class ScopeView {
  constructor(canvas) {
    this.ctx = canvas.getContext("2d");
    this.canvas = canvas;
  }
  
  draw(samples1, samples2, triggerThreshold) {
    const { ctx, canvas } = this;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.strokeStyle = "white";
    ctx.beginPath();
    ctx.moveTo(0, canvas.height / 2);
    ctx.lineTo(canvas.width, canvas.height / 2);
    ctx.stroke();
    
    // very basic plot
    if (samples1 && samples1.length > 0) {
      ctx.strokeStyle = "cyan";
      ctx.beginPath();
      for (let i = 0; i < samples1.length; i += 2) {
        const x = (i / samples1.length) * canvas.width;
        const y = canvas.height / 2 - samples1[i+1] * 10;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
    }
  }
}