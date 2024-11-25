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

window.URL_CLEANER_SITE = "http://localhost:9149";
window.PARAMS_DIFF = {"vars": {"SOURCE_URL": window.location.href, "SOURCE_HOST": window.location.hostname}};

(async () => {await GM_xmlhttpRequest({
	url: `${window.URL_CLEANER_SITE}/get-max-json-size`,
	onload: function(response) {
		window.MAX_JSON_SIZE = parseInt(response.responseText);
	}
})})();

function elements_to_bulk_job(elements) {
	return {urls: elements.map(x => element_to_job_config(x)), params_diff: window.PARAMS_DIFF};
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
			e.getAttribute("url-cleaner") == null);
	a: if (elements.length > 0) {
		// Limit total size of request. Repeated iterations will get all link elements.
		while (JSON.stringify(elements_to_bulk_job(elements)).length > window.MAX_JSON_SIZE) {
			if (elements.length == 1) {
				// If, somehow, there's a URL that's over 10MaB, this stops it from getting stuck in an infinite loop.
				elements[0].setAttribute("url-cleaner", "client-error");
				elements[0].setAttribute("url-cleaner-error", "URL Too long.");
				elements[0].style.color = "red";
				break a;
			} else {
				elements = elements.slice(0, elements.length/2);
			}
		}

		// `elements.length` should never be `0` at this point.
		// It shouldn't actually matter but if it happens it is an error.

		await clean_elements(elements);
	}
	setTimeout(clean_all_urls_on_page, 500);
}

async function clean_elements(elements) {
	let bulk_job = elements_to_bulk_job(elements);
	// Fuck CORS. I get why it exists and I appreciate it but it is so annoying.
	await GM_xmlhttpRequest({
		url: `${window.URL_CLEANER_SITE}/clean`,
		method: "POST",
		data: JSON.stringify(bulk_job),
		onload: function(response) {
			let result = JSON.parse(response.responseText);
			if (result.Err == null) {
				result.Ok.urls.forEach(function (cleaning_result, index) {
					if (cleaning_result.Err == null) {
						if (cleaning_result.Ok.Err == null) {
							if (elements[index].href != cleaning_result.Ok.Ok) {
								elements[index].href = cleaning_result.Ok.Ok;
								elements[index].setAttribute("url-cleaner", "success");
							} else {
								elements[index].setAttribute("url-cleaner", "unchanged");
							}
						} else {
							console.error("URL Cleaner DoJobError:", cleaning_result, "Element indesx:", index, "Element:", elements[index], "Job:", bulk_job[index]);
							elements[index].setAttribute("url-cleaner", "DoJobError");
							elements[index].setAttribute("url-cleaner-error", JSON.stringify(cleaning_result.Ok.Err));
							elements[index].style.color = "red";
						}
					} else {
						console.error("URL Cleaner MakeJobError:", cleaning_result, "Element indesx:", index, "Element:", elements[index], "Job:", bulk_job[index]);
						elements[index].setAttribute("url-cleaner", "MakeJobError");
						elements[index].setAttribute("url-cleaner-error", JSON.stringify(cleaning_result.Err));
						elements[index].style.color = "red";
					}
				})
			} else {
				console.error("URL Cleaner bulk job error", result);
			}
		}
	});
}

new MutationObserver(function(mutations) {
	mutations.forEach(async function(mutation) {
		if (mutation.target.hasAttribute("url-cleaner")) {
			mutation.target.removeAttribute("url-cleaner");
			mutation.target.removeAttribute("url-cleaner-error");
			if (mutation.target.matches(":hover, :active, :focus, :focus-visible, :focus-within")) {
				await clean_elements([mutation.target]);
			}
		}
	});
}).observe(document.querySelector("html"), {
	attributes: true,
	attributeFilter: ["href"],
	subtree: true
});

(async () => {await clean_all_urls_on_page()})();
