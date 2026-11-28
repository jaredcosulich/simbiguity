export function boot(mainFn) {
  let missing = [];
  if (typeof WebAssembly !== "object") missing.push("WebAssembly");
  const canvas = document.createElement("canvas");
  if (!canvas.getContext("webgl2")) missing.push("WebGL2");
  
  if (missing.length > 0) {
    showError(`Your browser doesn't support ${missing.join(" and ")}.`, "This demo requires them to run the engine and render the simulation.");
    return Promise.resolve();
  }

  return mainFn().catch(err => {
    showError("Application Error", err.message || String(err));
    console.error(err);
  });
}

function showError(title, message) {
  const errDiv = document.createElement("div");
  errDiv.style.padding = "2rem";
  errDiv.style.margin = "2rem auto";
  errDiv.style.maxWidth = "40rem";
  errDiv.style.borderLeft = "4px solid #d9480f";
  errDiv.style.background = "color-mix(in srgb, CanvasText 5%, Canvas)";
  errDiv.innerHTML = `
    <h2 style="margin-top: 0; color: #d9480f;">${title}</h2>
    <p style="margin-bottom: 0;">${message}</p>
  `;
  const canvas = document.querySelector("canvas");
  if (canvas) {
    canvas.replaceWith(errDiv);
  } else {
    document.body.prepend(errDiv);
  }
}
