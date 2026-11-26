export function renderBench(bench) {
  if (!bench || !bench.length) return [];
  
  const h2 = document.createElement("h2");
  h2.id = "benchmarks";
  h2.textContent = "Benchmarks";
  
  const p = document.createElement("p");
  p.className = "section-lede";
  p.textContent = "Execution time for critical engine paths, measured on deploy.";
  
  const table = document.createElement("table");
  table.className = "bench";
  const thead = document.createElement("thead");
  thead.innerHTML = "<tr><th>Benchmark</th><th class='num'>Median (µs)</th><th class='num'>95th Percentile (µs)</th></tr>";
  table.appendChild(thead);
  
  const tbody = document.createElement("tbody");
  for (const result of bench) {
    const tr = document.createElement("tr");
    
    const tdName = document.createElement("td");
    tdName.textContent = result.name;
    tr.appendChild(tdName);
    
    const tdMedian = document.createElement("td");
    tdMedian.className = "num";
    tdMedian.textContent = result.medianUs;
    tr.appendChild(tdMedian);
    
    const tdP95 = document.createElement("td");
    tdP95.className = "num";
    tdP95.textContent = result.p95Us;
    tr.appendChild(tdP95);
    
    tbody.appendChild(tr);
  }
  table.appendChild(tbody);
  
  return [h2, p, table];
}