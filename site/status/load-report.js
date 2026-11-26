// Fetch the report the status page renders.

/** The parsed report at `url`; throws with the HTTP status on failure. */
export async function loadReport(url) {
  const res = await fetch(url, { cache: "no-store" });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json();
}
