// Load each demo iframe only when its card scrolls near the viewport.

const observer = "IntersectionObserver" in window
  ? new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          const frame = entry.target.querySelector("iframe[data-src]");
          if (frame) {
            frame.src = frame.dataset.src;
            frame.removeAttribute("data-src");
          }
          observer.unobserve(entry.target);
        }
      },
      { rootMargin: "200px" },
    )
  : null;

/** Start loading `frame` (whose `data-src` holds the URL) once `box` is near. */
export function observeDemo(box, frame) {
  if (observer) observer.observe(box);
  else frame.src = frame.dataset.src;
}
