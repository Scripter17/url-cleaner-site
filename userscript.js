// ==UserScript==
// @name         URL Cleaner
// @copyright    AGPL-3.0-or-later
// @version      0.8.0
// @description  The userscript that comes with URL Cleaner Site.
// @author       Scripter17@Github.com
// @match        https://*/*
// @match        http://*/*
// @grant        GM.xmlHttpRequest
// @connect      localhost
// @grant        GM.setValue
// @grant        GM.getValue
// ==/UserScript==

window.URL_CLEANER_SITE = "http://localhost:9149";
window.JOBS_CONTEXT     = {"vars": {"SOURCE_URL": window.location.href}}; // "SOURCE_REG_DOMAIN" is added in the init at the bottom.
window.PARAMS_DIFF      = null;

window.debug = {
	new_bulk_job     : false,
	api_request_info : false,
	api_request_error: true,
	api_response_info: false,
	other_timing_info: false,
	new_reg_domain   : false,
	href_mutations   : false,
	max_bulk_job_size: false
};

window.cleaned_elements = new WeakMap();
window.too_big_elements = new WeakSet();
window.errored_elements = new WeakSet();
window.total_elements_cleaned = 0;
window.total_time_cleaning = 0;

function elements_to_bulk_job(elements) {
	let ret = {
		jobs: elements.map(x => element_to_job_config(x)),
		context: window.JOBS_CONTEXT
	};
	if (window.PARAMS_DIFF) {ret.params_diff = window.PARAMS_DIFF;}
	return ret;
}

function element_to_job_config(element) {
	if (window.JOBS_CONTEXT.vars.SOURCE_REG_DOMAIN == "x.com" && element.href.startsWith("https://t.co/") && element.innerText.startsWith("http")) {
		// On twitter, links in tweets/bios/whatever show the entire URL when you hover over them for a moemnt.
		// This lets us skip the HTTP request to t.co for the vast majority of links on twitter.
		return {
			url: element.href,
			context: {
				vars: {
					redirect_shortcut: element.childNodes[0].innerText + (element.childNodes[1].textContent) + (element.childNodes[2]?.innerText ?? "")
				}
			}
		};
	} else if (window.JOBS_CONTEXT.vars.SOURCE_REG_DOMAIN == "allmylinks.com" && element.pathname=="/link/out" && element.title.startsWith("http")) {
		return {
			url: element.href,
			context: {
				vars: {
					redirect_shortcut: element.title
				}
			}
		};
	} else if (window.JOBS_CONTEXT.vars.SOURCE_REG_DOMAIN == "furaffinity.net" && element.matches(".user-contact-user-info a") && element.innerText != encodeURIComponent(element.innerText)) {
		/// Allows unmangling contact info links.
		return {
			url: decodeURIComponent(element.href),
			context: {
				vars: {
					site_name: element.parentElement.querySelector("strong").innerHTML,
					link_text: element.innerText
				}
			}
		};
	} else {
		return element.href;
	}
}

async function main_loop() {
	var elements = [...document.links]
		.filter(e => !e.getAttribute("href").startsWith("#") && // Websites often use `href="#"` to make buttons work on slop browsers like ios safari.
			!window.cleaned_elements.has(e) && !window.too_big_elements.has(e) && !window.errored_elements.has(e) // Make sure we didn't already handle it.
		);
	await clean_elements(elements);
	setTimeout(main_loop, 100); // Is this a good interval? No idea. Is an interval even the right approach? Also no idea.
}

// The `bulk_jobs` parameter is used to make breaking big jobs into parts faster. I think.
async function clean_elements(elements) {

	let bulk_job = elements_to_bulk_job(elements);

	// If the job is too bulky, break it into parts.
	if (JSON.stringify(bulk_job).length > window.MAX_JSON_SIZE) {
		if (elements.length == 1) {
			// If, somehow, there's a URL that's over the server's size limit, this stops it from getting stuck in an infinite loop.
			console.error(`[URLC] URL Cleaner element too big error: ${elements[0]}`);
			window.too_big_elements.add(elements[0]);
			return;
		} else {
			/// Cut the list in half and do them separately.
			await clean_elements(elements.slice(0, elements.length/2), {...bulk_job, jobs: bulk_job.slice(0, bulk_job.jobs.length/2)});
			elements = elements.slice(elements.length/2);
			bulk_job.jobs = bulk_job.jobs.slice(bulk_job.jobs.length/2);
		}
	}

	if (bulk_job.jobs.length == 0) {return;}

	let start_time = new Date();
	let id = Math.floor(Math.random()*1e8); // Random to avoid iframes from being confusing.
	let id_pad = " ".repeat(8-id.toString().length)
	let last_time = start_time;
	let now;
	let data = JSON.stringify(bulk_job);
	let done;
	let doneawaiter = new Promise(resolve => {done = resolve;});
	if (window.debug.new_bulk_job) {console.log("[URLC]"+id_pad, id, elements.length, "elements in", data.length, "bytes (", bulk_job, ")");}
	// This returns `undefined` in GreaseMonkey, so the weird "await for callback" pattern is required.
	await GM.xmlHttpRequest({
		url: `${window.URL_CLEANER_SITE}/clean`,
		method: "POST",
		data: data,
		timeout: 10000,
		onabort           : (e) => {if (window.debug.api_request_error) {now = new Date(); console.error("[URLC]"+id_pad, id, "abort            took", now-last_time, "ms (", e, ")"); last_time = now;} done();},
		onerror           : (e) => {if (window.debug.api_request_error) {now = new Date(); console.error("[URLC]"+id_pad, id, "error            took", now-last_time, "ms (", e, ")"); last_time = now;} done();},
		onloadstart       : (e) => {if (window.debug.api_request_info ) {now = new Date(); console.log  ("[URLC]"+id_pad, id, "loadstart        took", now-last_time, "ms (", e, ")"); last_time = now;}},
		onloadprogress    : (e) => {if (window.debug.api_request_info ) {now = new Date(); console.log  ("[URLC]"+id_pad, id, "loadprogress     took", now-last_time, "ms (", e, ")"); last_time = now;}},
		onreadystatechange: (e) => {if (window.debug.api_request_info ) {now = new Date(); console.log  ("[URLC]"+id_pad, id, "readystatechange took", now-last_time, "ms (", e, ")"); last_time = now;}},
		ontimeout         : (e) => {if (window.debug.api_request_error) {now = new Date(); console.error("[URLC]"+id_pad, id, "timeout          took", now-last_time, "ms (", e, ")"); last_time = now;} done();},
		onload: function(response) {
			if (window.debug.api_response_info) {now = new Date(); console.log("[URLC]"+id_pad, id, "load             took", now-last_time, "ms (", response, ")"); last_time = now;}
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
							console.error("[URLC]"+id_pad, id, "DoJobError:", cleaning_result.Ok.Err, "Element indesx:", index, "Element:", elements[index], "Job:", bulk_job.jobs[index]);
							window.errored_elements.add(elements[index])
						}
					} else {
						console.error("[URLC]"+id_pad, id, "MakeJobError:", cleaning_result.Err, "Element indesx:", index, "Element:", elements[index], "Job:", bulk_job.jobs[index]);
						window.errored_elements.add(elements[index])
					}
				});
			} else {
				console.error("[URLC]"+id_pad, id, "bulk job error", result);
			}
			now = new Date();
			window.total_time_cleaning += now-start_time;
			window.total_elements_cleaned += elements.length;
			if (window.debug.other_timing_info) {console.log("[URLC]"+id_pad, id, "writing          took", now-last_time , "ms");}
			if (window.debug.other_timing_info) {console.log("[URLC]"+id_pad, id, "all              took", now-start_time, "ms");}
			if (window.debug.other_timing_info) {console.log("[URLC]", "Total cleaning took", window.total_time_cleaning, "ms for", window.total_elements_cleaned, "elements");}
			done();
		}
	});
	await doneawaiter;
}

(async () => {
	console.log("[URLC] URL Cleaner Site Userscript loaded. Please note that initial cleanings take a long time because there's a lot happening.");

	// For reasons I don't understand, awaiting `GM.xmlHttpRequest` doesn't seem to, uh, await it.
	// It might be me being stupid.
	let done;
	let doneawaiter = new Promise(resolve => {done = resolve;});
	await GM.xmlHttpRequest({
		url: `${window.URL_CLEANER_SITE}/get-max-json-size`,
		method: "GET",
		onload: function(response) {
			window.MAX_JSON_SIZE = parseInt(response.responseText);
			done();
		}
	});
	await doneawaiter;

	doneawaiter = new Promise(resolve => {done = resolve;});
	let host_details = await GM.getValue(`host-details-of-${window.location.hostname}`);
	a: if (!host_details) {
		// Check the very jank cache for some suffix of the current domain that is a RegDomain.
		// For `abc.def.example.com`, it checks `abc.def.example.com`, `def.example.com`, `example.com`, and `com`.
		let parts = window.location.hostname.replace(/\.$/g, "").split('.');
		for (let i=0; i<parts.length; i++) {
			let n_parent_domain = parts.slice(i).join(".");
			if (await GM.getValue(`${n_parent_domain}-is-reg-domain`)) {
				done(n_parent_domain);
				break a;
			}
		}

		// As the log implies, we only reach here when the cache doesn't have a RegDomain for the current domain, so we're asking URL Cleaner Site.
		// I think in theory this can be done client-side using `document.cookie` but uh... no.
		if (window.debug.new_reg_domain) {console.log(`[URLC] Couldn't find the RegDomain for ${window.location.hostname}. Asking URL Cleaner Site...`);}

		await GM.xmlHttpRequest({
			url: `${window.URL_CLEANER_SITE}/host-parts`,
			method: "POST",
			data: window.location.hostname,
			onload: async function(response) {
				await GM.setValue(`host-details-of-${window.location.hostname}`, response.responseText);
				let reg_domain = JSON.parse(response.responseText)?.Ok?.Domain?.reg_domain;
				if (reg_domain) {await GM.setValue(`${reg_domain}-is-reg-domain`, reg_domain != null);}
				done(reg_domain);
			}
		})
	} else {
		// `?.` is a great operator.
		// `null.thing` throws an error but `null?.thing` just returns `null`.
		done(JSON.parse(host_details)?.Ok?.Domain?.reg_domain);
	}
	let reg_domain = await doneawaiter;

	if (reg_domain) {
		window.JOBS_CONTEXT.vars.SOURCE_REG_DOMAIN = reg_domain;
	}

	// Some websites change URLs when you, for example, mousedown on them.
	// If you left click it, this waits for it to be cleaned.
	new MutationObserver(function(mutations) {
		if (window.debug.href_mutations) {console.log("[URLC]", "Href mutations observed (", mutations, ")");}
		mutations.forEach(function(mutation) {
			if (window.cleaned_elements.get(mutation.target) != mutation.target.href) {
				window.cleaned_elements.delete(mutation.target);
				window.too_big_elements.delete(mutation.target);
				window.errored_elements.delete(mutation.target);
				if (mutation.target.matches(":hover, :active, :focus, :focus-visible, :focus-within")) {
					mutation.target.addEventListener("click", async function(e) {
						if (window.cleaned_elements.has(e.target) || window.too_big_elements.has(e.target) || window.errored_elements.has(e.target)) {return;}
						e.preventDefault();
						await clean_elements([e.target]);
						e.target.click();
					}, {capture: true, once: true});
				}
			}
		});
	}).observe(document.documentElement, {
		attributes: true,
		attributeFilter: ["href"],
		subtree: true
	});

	if (window.debug.max_bulk_job_size) {console.log("[URLC] max bulk job size is", window.MAX_JSON_SIZE, "bytes");}
	await main_loop();
})();
