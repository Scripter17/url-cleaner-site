// ==UserScript==
// @name         URL Cleaner
// @namespace    http://tampermonkey.net/
// @version      2024-04-16
// @description  Quick and dirty URL Cleaner userscript.
// @author       Scripter17@Github.com
// @match        https://*/*
// @match        http://*/*
// @grant        GM_xmlhttpRequest
// @connect      localhost
// ==/UserScript==

async function doit() {
	let elements = [...document.getElementsByTagName("a")]
		.filter(e => e.href.startsWith("http") && e.getAttribute("url-cleaned") == null);
	if (elements.length > 0) {
		await GM_xmlhttpRequest({
			url: "http://localhost:9149/clean",
			method: "POST",
			data: JSON.stringify({urls: elements.map(x => x.href)}),
			onload: function(response) {
				JSON.parse(response.responseText).urls.forEach(function (cleaning_result, index) {
					if (cleaning_result.Err == null) {
						if (elements[index].href != cleaning_result.Ok) {elements[index].href = cleaning_result.Ok;}
						elements[index].setAttribute("url-cleaned", "true");
					} else {
						console.log("URL Cleaner error": cleaning_result, index, cleaning_result);
						elements[index].setAttribute("url-cleaned", "error");
						elements[index].setAttribute("url-cleaner-error", cleaning_result.Err);
						elements[index].style.color = "red";
					}
				})
			}
		});
	}
}

await doit();
setInterval(doit, 500);
