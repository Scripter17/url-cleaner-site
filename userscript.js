// ==UserScript==
// @name         URL Cleaner
// @copyright    AGPL-3.0-or-later
// @namespace    https://github.com/Scripter17/url-cleaner-site
// @version      2024-11-29
// @description  The userscript that comes with URL Cleaner Site.
// @author       Scripter17@Github.com
// @match        https://*/*
// @match        http://*/*
// @grant        GM.xmlHttpRequest
// @connect      localhost
// ==/UserScript==

window.URL_CLEANER_SITE = "http://localhost:9149";
window.PARAMS_DIFF = {"vars": {"SOURCE_URL": window.location.href}};

window.debug = 0;

(async () => {await GM.xmlHttpRequest({
	url: `${window.URL_CLEANER_SITE}/get-max-json-size`,
	onload: function(response) {
		window.MAX_JSON_SIZE = parseInt(response.responseText);
	}
})})();

window.cleaned_elements = new WeakSet();
window.too_big_elements = new WeakSet();
window.errored_elements = new WeakSet();

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
			!window.cleaned_elements.has(e) && !window.too_big_elements.has(e) && !window.errored_elements.has(e));
	a: if (elements.length > 0) {
		// Limit total size of request. Repeated iterations will get all link elements.
		while (JSON.stringify(elements_to_bulk_job(elements)).length > window.MAX_JSON_SIZE) {
			if (elements.length == 1) {
				// If, somehow, there's a URL that's over 10MaB, this stops it from getting stuck in an infinite loop.
				console.error(`URL Cleaner element too big error: ${elements[0]}`);
				window.too_big_elements.add(elements[0])
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
	let start_time = new Date();
  let id = Math.random();
	let last_time = start_time;
	let now;
	let data = JSON.stringify(bulk_job);
  let done;
  let donedetector = new Promise(resolve => {done = resolve});
	if (window.debug >= 1) {console.log("[URLC]", id, elements.length, "elements in", data.length, "bytes.");}
	// This returns `undefined` in GreaseMonkey, so the weird "await for callback" pattern is required.
	await GM.xmlHttpRequest({
		url: `${window.URL_CLEANER_SITE}/clean`,
		method: "POST",
		data: data,
		onabort           : (x) => {if (window.debug >= 2) {now = new Date(); console.log("[URLC]", id, "abort            took ", now-last_time, "ms", x); last_time = now;}},
		onerror           : (x) => {if (window.debug >= 2) {now = new Date(); console.log("[URLC]", id, "error            took ", now-last_time, "ms", x); last_time = now;}},
		onloadstart       : (x) => {if (window.debug >= 2) {now = new Date(); console.log("[URLC]", id, "loadstart        took ", now-last_time, "ms", x); last_time = now;}},
		onloadprogress    : (x) => {if (window.debug >= 2) {now = new Date(); console.log("[URLC]", id, "loadprogress     took ", now-last_time, "ms", x); last_time = now;}},
		onreadystatechange: (x) => {if (window.debug >= 2) {now = new Date(); console.log("[URLC]", id, "readystatechange took ", now-last_time, "ms", x); last_time = now;}},
		ontimeout         : (x) => {if (window.debug >= 2) {now = new Date(); console.log("[URLC]", id, "timeout          took ", now-last_time, "ms", x); last_time = now;}},
		onload: function(response) {
			if (window.debug == 1) {now = new Date(); console.log("[URLC]", id, "load             took ", now-last_time, "ms"); last_time = now;}
			if (window.debug >= 2) {now = new Date(); console.log("[URLC]", id, "load             took ", now-last_time, "ms", response); last_time = now;}
			let result = JSON.parse(response.responseText);
			if (result.Err == null) {
				result.Ok.urls.forEach(function (cleaning_result, index) {
					if (cleaning_result.Err == null) {
						if (cleaning_result.Ok.Err == null) {
							if (elements[index].href != cleaning_result.Ok.Ok) {
								elements[index].setAttribute("href", cleaning_result.Ok.Ok);
							}
							window.cleaned_elements.add(elements[index]);
						} else {
							console.error("URL Cleaner DoJobError:", cleaning_result, "Element indesx:", index, "Element:", elements[index], "Job:", bulk_job[index]);
							window.errored_elements.add(elements[index])
						}
					} else {
						console.error("URL Cleaner MakeJobError:", cleaning_result, "Element indesx:", index, "Element:", elements[index], "Job:", bulk_job[index]);
						window.errored_elements.add(elements[index])
					}
				});
			} else {
				console.error("URL Cleaner bulk job error", result);
			}
			if (window.debug >= 2) {now = new Date(); console.log("[URLC]", id, "writing          took ", now-last_time , "ms"); last_time = now;}
			if (window.debug >= 2) {now = new Date(); console.log("[URLC]", id, "all              took ", now-start_time, "ms");}
	    done();
		}
	});
  await donedetector;
  console.log("cleaned");
}

async function interrupt_click_until_clean(e) {
	e.preventDefault();
	await clean_elements([e.target]);
	e.target.click();
}

new MutationObserver(async function(mutations) {
	mutations.forEach(function(mutation) {
		window.cleaned_elements.delete(mutation.target);
		window.too_big_elements.delete(mutation.target);
		window.errored_elements.delete(mutation.target);
		if (mutation.target.matches(":hover, :active, :focus, :focus-visible, :focus-within")) {
			mutation.target.addEventListener("click", interrupt_click_until_clean, {capture: true, once: true});
		}
	});
}).observe(document.querySelector("html"), {
	attributes: true,
	attributeFilter: ["href"],
	subtree: true
});

(async () => {await clean_all_urls_on_page()})();
