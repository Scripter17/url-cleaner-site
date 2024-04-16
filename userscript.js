// ==UserScript==
// @name         URL Cleaner
// @namespace    http://tampermonkey.net/
// @version      2024-04-16
// @description  Quick, dirty, and shitty URL Cleaner userscript. Definitely has problems with shortlinks.
// @author       You
// @match        https://*/*
// @match        http://*/*
// @grant        GM_xmlhttpRequest
// @connect      localhost
// ==/UserScript==

async function doit() {
	let elements = [...document.getElementsByTagName("a")]
		.filter(e => e.href.startsWith("http") && e.getAttribute("url-cleaned") != "true");
	if (elements.length > 0) {
		await GM_xmlhttpRequest({
			url: "http://localhost:9149/clean",
			method: "POST",
			data: JSON.stringify({urls: elements.map(x => x.href)}),
			onload: function(response) {
				JSON.parse(response.responseText).urls.forEach(function (cleaned_url, index) {
					elements[index].href = cleaned_url;
					elements[index].setAttribute("url-cleaned", "true");
				})
			}
		});
	}
}

await doit();
setInterval(doit, 500);
