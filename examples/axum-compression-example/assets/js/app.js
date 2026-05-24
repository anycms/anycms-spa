document.getElementById("app").textContent = "JS loaded successfully!";

// Self-test: check response headers via fetch
(async function () {
    const results = [];

    // Test 1: Brotli compression
    const brResp = await fetch("/css/style.css", { headers: { "Accept-Encoding": "br,gzip" } });
    const brEncoding = brResp.headers.get("Content-Encoding");
    results.push(`Brotli: ${brEncoding === "br" ? "PASS" : "FAIL"} (got: ${brEncoding})`);

    // Test 2: Gzip fallback
    const gzipResp = await fetch("/css/style.css", { headers: { "Accept-Encoding": "gzip" } });
    const gzipEncoding = gzipResp.headers.get("Content-Encoding");
    results.push(`Gzip: ${gzipEncoding === "gzip" ? "PASS" : "FAIL"} (got: ${gzipEncoding})`);

    // Test 3: Security headers
    const secResp = await fetch("/");
    const xcto = secResp.headers.get("X-Content-Type-Options");
    const xfo = secResp.headers.get("X-Frame-Options");
    results.push(`Security: ${xcto === "nosniff" && xfo === "SAMEORIGIN" ? "PASS" : "FAIL"}`);

    // Test 4: Vary header
    const vary = secResp.headers.get("Vary");
    results.push(`Vary: ${vary === "Accept-Encoding" ? "PASS" : "FAIL"} (got: ${vary})`);

    // Test 5: Charset
    const ct = secResp.headers.get("Content-Type");
    results.push(`Charset: ${ct.includes("charset=utf-8") ? "PASS" : "FAIL"} (got: ${ct})`);

    // Test 6: Custom 404
    const notFound = await fetch("/nonexistent-page");
    const isHtml = notFound.headers.get("Content-Type")?.includes("text/html");
    results.push(`Custom 404: ${notFound.status === 404 && isHtml ? "PASS" : "FAIL"} (status: ${notFound.status})`);

    // Test 7: Accept-Ranges header
    const acceptRanges = secResp.headers.get("Accept-Ranges");
    results.push(`Accept-Ranges: ${acceptRanges === "bytes" ? "PASS" : "FAIL"} (got: ${acceptRanges})`);

    // Test 8: Range request (partial content)
    try {
        const rangeResp = await fetch("/css/style.css", { headers: { "Range": "bytes=0-49" } });
        const rangeStatus = rangeResp.status;
        const contentRange = rangeResp.headers.get("Content-Range") || "";
        const rangeData = await rangeResp.text();
        const rangeOk = rangeStatus === 206 && contentRange.startsWith("bytes 0-49/") && rangeData.length === 50;
        results.push(`Range 206: ${rangeOk ? "PASS" : "FAIL"} (status: ${rangeStatus}, Content-Range: ${contentRange}, body len: ${rangeData.length})`);
    } catch (e) {
        results.push(`Range 206: FAIL (${e.message})`);
    }

    // Test 9: If-Range with wrong ETag -> full content
    try {
        const ifRangeResp = await fetch("/css/style.css", {
            headers: { "Range": "bytes=0-49", "If-Range": '"wrong-etag"' }
        });
        const fullContent = ifRangeResp.status === 200;
        results.push(`If-Range mismatch: ${fullContent ? "PASS" : "FAIL"} (status: ${ifRangeResp.status}, expected 200)`);
    } catch (e) {
        results.push(`If-Range mismatch: FAIL (${e.message})`);
    }

    const ul = document.createElement("ul");
    results.forEach(r => {
        const li = document.createElement("li");
        li.textContent = r;
        ul.appendChild(li);
    });
    const div = document.createElement("div");
    div.className = "test-results";
    div.innerHTML = "<h2>Feature Tests</h2>";
    div.appendChild(ul);
    document.body.appendChild(div);
})();
