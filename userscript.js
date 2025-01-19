// ==UserScript==
// @name         URL Cleaner
// @copyright    AGPL-3.0-or-later
// @namespace    https://github.com/Scripter17/url-cleaner-site
// @version      0.7.1
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

window.cleaned_elements = new WeakMap();
window.too_big_elements = new WeakSet();
window.errored_elements = new WeakSet();
window.total_elements_cleaned = 0;
window.total_time_cleaning = 0;

function elements_to_bulk_job(elements) {
	return {urls: elements.map(x => element_to_job_config(x)), params_diff: window.PARAMS_DIFF};
}

function element_to_job_config(element) {
	if (window.location.hostname == "x.com" && element.href.startsWith("https://t.co/") && element.innerText.startsWith("http")) {
		// On twitter, links in tweets/bios/whatever show the entire URL when you hover over them for a moemnt.
		// This lets us skip the HTTP request to t.co for the vast majority of links on twitter.
		return {
			url: element.href,
			context: {
				vars: {
					alt_text: element.childNodes[0].innerText + (element.childNodes[1].textContent) + (element.childNodes[2]?.innerText ?? "")
				}
			}
		};
	} else {
		return element.href;
	}
}

async function clean_all_urls_on_page() {
	var elements = [...document.links]
		.filter(e => e.href.startsWith("http") && // Relative URLs are replaced with absolute URLs when getting the `href` property. Also cleaning "javscript:void(0)" returns an error for some reason.
		!window.cleaned_elements.has(e) && !window.too_big_elements.has(e) && !window.errored_elements.has(e));
	a: if (elements.length > 0) {
		// Limit total size of request. Repeated iterations will get all link elements.
		while (JSON.stringify(elements_to_bulk_job(elements)).length > window.MAX_JSON_SIZE) {
			if (elements.length == 1) {
				// If, somehow, there's a URL that's over the server's size limit, this stops it from getting stuck in an infinite loop.
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
	setTimeout(clean_all_urls_on_page, 100);
}

// If the call to GM.xmlHttpRequest aborts, errors, or times out, returns false. Otherwise returns true.
async function clean_elements(elements) {
	let bulk_job = elements_to_bulk_job(elements);
	let start_time = new Date();
  let id = Math.floor(Math.random()*1e8); // Random to avoid iframes from being confusing.
  let id_pad = " ".repeat(8-id.toString().length)
	let last_time = start_time;
	let now;
	let data = JSON.stringify(bulk_job);
  let done;
  let doneawaiter = new Promise(resolve => {done = resolve;});
	if (window.debug >= 1) {console.log("[URLC]"+id_pad, id, elements.length, "elements in", data.length, "bytes");}
	// This returns `undefined` in GreaseMonkey, so the weird "await for callback" pattern is required.
	await GM.xmlHttpRequest({
		url: `${window.URL_CLEANER_SITE}/clean`,
		method: "POST",
		data: data,
		timeout: 10000,
		onabort           : (e) => {if (window.debug >= 1) {now = new Date(); loggables = ["[URLC]"+id_pad, id, "abort            took", now-last_time, "ms"]; if (window.debug >= 3) {loggables.push(e)} console.error(...loggables); last_time = now;} done(false);},
		onerror           : (e) => {if (window.debug >= 1) {now = new Date(); loggables = ["[URLC]"+id_pad, id, "error            took", now-last_time, "ms"]; if (window.debug >= 3) {loggables.push(e)} console.error(...loggables); last_time = now;} done(false);},
		onloadstart       : (e) => {if (window.debug >= 2) {now = new Date(); loggables = ["[URLC]"+id_pad, id, "loadstart        took", now-last_time, "ms"]; if (window.debug >= 3) {loggables.push(e)} console.log  (...loggables); last_time = now;}},
		onloadprogress    : (e) => {if (window.debug >= 2) {now = new Date(); loggables = ["[URLC]"+id_pad, id, "loadprogress     took", now-last_time, "ms"]; if (window.debug >= 3) {loggables.push(e)} console.log  (...loggables); last_time = now;}},
		onreadystatechange: (e) => {if (window.debug >= 2) {now = new Date(); loggables = ["[URLC]"+id_pad, id, "readystatechange took", now-last_time, "ms"]; if (window.debug >= 3) {loggables.push(e)} console.log  (...loggables); last_time = now;}},
		ontimeout         : (e) => {if (window.debug >= 1) {now = new Date(); loggables = ["[URLC]"+id_pad, id, "timeout          took", now-last_time, "ms"]; if (window.debug >= 3) {loggables.push(e)} console.error(...loggables); last_time = now;} done(false);},
		onload: function(response) {
			if (window.debug >= 1) {now = new Date(); let loggables = ["[URLC]"+id_pad, id, "load             took", now-last_time, "ms"]; if (window.debug >=3) {loggables.push(response);} console.log(...loggables); last_time = now;}
			let result = JSON.parse(response.responseText);
			if (result.Err == null) {
				result.Ok.urls.forEach(function (cleaning_result, index) {
					if (cleaning_result.Err == null) {
						if (cleaning_result.Ok.Err == null) {
							if (elements[index].href != cleaning_result.Ok.Ok) {
								elements[index].setAttribute("href", cleaning_result.Ok.Ok);
							}
							window.cleaned_elements.set(elements[index], cleaning_result.Ok.Ok);
						} else {
							console.error("[URLC]"+id_pad, id, "DoJobError:", cleaning_result.Ok.Err, "Element indesx:", index, "Element:", elements[index], "Job:", bulk_job[index]);
							window.errored_elements.add(elements[index])
						}
					} else {
						console.error("[URLC]"+id_pad, id, "MakeJobError:", cleaning_result.Err, "Element indesx:", index, "Element:", elements[index], "Job:", bulk_job[index]);
						window.errored_elements.add(elements[index])
					}
				});
			} else {
				console.error("[URLC]"+id_pad, id, "bulk job error", result);
			}
			now = new Date();
			window.total_time_cleaning += now-start_time;
			window.total_elements_cleaned += elements.length;
			if (window.debug >= 1) {console.log("[URLC]"+id_pad, id, "writing          took", now-last_time , "ms");}
			if (window.debug >= 2) {console.log("[URLC]"+id_pad, id, "all              took", now-start_time, "ms");}
			if (window.debug >= 1) {console.log("[URLC]", "total cleaning took", window.total_time_cleaning, "ms for", window.total_elements_cleaned, "elements");}
	    done(true);
		}
	});
  return await doneawaiter;
}

async function interrupt_click_until_clean(e) {
	if (window.cleaned_elements.delete(e) || window.too_big_elements.delete(e) || window.errored_elements.delete(e)) {return;}
	e.preventDefault();
	await clean_elements([e.target]);
	e.target.click();
}

(async () => {
	console.log("[URLC] URL Cleaner Site Userscript loaded. Please note that initial cleanings take a long time because there's a lot happening.");
	await GM.xmlHttpRequest({
		url: `${window.URL_CLEANER_SITE}/get-max-json-size`,
		method: "GET",
		onload: function(response) {
			window.MAX_JSON_SIZE = parseInt(response.responseText);
		}
	});

	// Some websites change URLs when you, for example, mousedown on them.
	// If you left click it, this waits for it to be cleaned.
	new MutationObserver(function(mutations) {
		if (window.debug >= 1) {console.log("[URLC]", "Href mutations observed");}
		mutations.forEach(function(mutation) {
			if (window.cleaned_elements.get(mutation.target) != mutation.target.href) {
				window.cleaned_elements.delete(mutation.target);
				window.too_big_elements.delete(mutation.target);
				window.errored_elements.delete(mutation.target);
				if (mutation.target.matches(":hover, :active, :focus, :focus-visible, :focus-within")) {
					mutation.target.addEventListener("click", interrupt_click_until_clean, {capture: true, once: true});
				}
			}
		});
	}).observe(document.querySelector("html"), {
		attributes: true,
		attributeFilter: ["href"],
		subtree: true
	});

	if (window.debug >= 1) {console.log("[URLC] max bulk job size is", window.MAX_JSON_SIZE, "bytes");}
	await clean_all_urls_on_page();
})();
