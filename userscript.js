// ==UserScript==
// @name         URL Cleaner
// @copyright    AGPL-3.0-or-later
// @namespace    https://github.com/Scripter17/url-cleaner-site
// @version      2024-04-16
// @description  Quick and dirty URL Cleaner userscript.
// @author       Scripter17@Github.com
// @match        https://*/*
// @match        http://*/*
// @grant        GM_xmlhttpRequest
// @connect      localhost
// ==/UserScript==

window.URL_CLEANER_SITE = "localhost:9149";
window.PARAMS_DIFF = {"vars": {"SOURCE_URL": window.location.href, "SOURCE_HOST": window.location.hostname}};

(async () => {await GM_xmlhttpRequest({
	url: `http://${window.URL_CLEANER_SITE}/get-max-json-size`,
	onload: function(response) {
		window.MAX_JSON_SIZE = parseInt(response.responseText);
	}
})})();

function elements_to_bulk_job(elements) {
	return {jobs: elements.map(x => element_to_job_config(x)), params_diff: window.PARAMS_DIFF};
}

function element_to_job_config(element) {
	if (window.location.hostname == "x.com" && element.href.startsWith("https://t.co/") && element.innerText.startsWith("http")) {
		return {
			url: element.href,
			context: {
				vars: {
					alt_text: element.childNodes[0].innerText + (element.childNodes[1].textContent) + (element.childNodes[2]?.innerText ?? "")
				}
			}
		}
	} else {
		return element.href
	}
}

async function clean_all_urls_on_page() {
	var elements = [...document.getElementsByTagName("a")]
		.filter(e => e.href.startsWith("http") && // Relative URLs are replaced with absolute URLs when getting the `href` property. Also cleaning "javscript:void(0)" returns an error for some reason.
			e.getAttribute("url-cleaned") == null);
	a: if (elements.length > 0) {
		// Limit total size of request. Repeated iterations will get all link elements.
		while (JSON.stringify(elements_to_bulk_job(elements)).length > window.MAX_JSON_SIZE) {
			if (elements.length == 1) {
				// If, somehow, there's a URL that's over 10MaB, this stops it from getting stuck in an infinite loop.
				elements[0].setAttribute("url-cleaned", "client-error");
				elements[0].setAttribute("url-cleaner-error", "URL Too long.");
				elements[0].style.color = "red";
				break a;
			} else {
				elements = elements.slice(0, elements.length/2);
			}
		}

		// `elements.length` should never be `0` at this point.
		// It shouldn't actually matter but if it happens it is an error.

		// Fuck CORS. I get why it exists and I appreciate it but it is so annoying.
		await GM_xmlhttpRequest({
			url: `http://${window.URL_CLEANER_SITE}/clean`,
			method: "POST",
			data: JSON.stringify(elements_to_bulk_job(elements)),
			onload: function(response) {
				JSON.parse(response.responseText).forEach(function (cleaning_result, index) {
					if (cleaning_result.Err == null) {
						if (elements[index].href != cleaning_result.Ok) {elements[index].href = cleaning_result.Ok;}
						elements[index].setAttribute("url-cleaned", "success");
					} else {
						console.error("URL Cleaner error:", cleaning_result, index, elements[index]);
						elements[index].setAttribute("url-cleaned", "response-error");
						elements[index].setAttribute("url-cleaner-error", cleaning_result.Err);
						elements[index].style.color = "red";
					}
				})
			}
		});
	}
	setTimeout(clean_all_urls_on_page, 500);
}

(async () => {await clean_all_urls_on_page()})();
