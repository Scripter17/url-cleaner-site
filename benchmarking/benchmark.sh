#!/usr/bin/bash

URLS=(\
  "https://x.com?a=2"\
  "https://example.com?fb_action_ids&mc_eid&ml_subscriber_hash&oft_ck&s_cid&unicorn_click_id"\
  "https://www.amazon.ca/UGREEN-Charger-Compact-Adapter-MacBook/dp/B0C6DX66TN/ref=sr_1_5?crid=2CNEQ7A6QR5NM&keywords=ugreen&qid=1704364659&sprefix=ugreen%2Caps%2C139&sr=8-5&ufe=app_do%3Aamzn1.fos.b06bdbbe-20fd-4ebc-88cf-fa04f1ca0da8"\
)
NUMS=0,1,10,100,1000,10000

rm -f *.out-*

compile=1
hyperfine=1
oha=1

COMMAND="curl --json @- http://localhost:9149/clean -f"

for arg in "$@"; do
  shift
  case "$arg" in
    "--no-compile") compile=0 ;;
    "--no-hyperfine") hyperfine=0 ;;
    "--no-oha") oha=0 ;;
    *) echo Unknwon option \"$arg\" && exit 1 ;;
  esac
done

if [ $compile -eq 1 ]; then cargo build -r; fi

if [ $? -ne 0 ]; then exit; fi

if [ $hyperfine -eq 1 ]; then
  touch stdin
  hyperfine \
    -L num $(echo "${NUMS[@]}" | sed "s/ /,/g") \
    -L url $(echo "${URLS[@]}" | sed "s/ /,/g") \
    --prepare "bash -c \"yes '\\\"{url}\\\"' | head -n {num} | jq -sc '{urls: .}' > stdin\"" \
    --max-runs 100 \
    --warmup 20 \
    --input stdin \
    -N \
    "$COMMAND" \
    --sort command \
    --export-json "hyperfine.out.json" \
    --command-name ""
  rm stdin
  cat hyperfine.out.json |\
    jq 'reduce .results[] as $result ({}; .[$result.parameters.url][$result.parameters.num] = ($result.mean * 1000000 | floor / 1000 | tonumber))' |\
    sed -E ":a /^    .{0,7}\s\S/ s/:/: /g ; ta :b /^    .{,11}\./ s/:/: /g ; tb ; :c /^    .+\..{0,2}(,|$)/ s/,|$/0&/g ; tc" |\
    tee hyperfine.out-summary.json |\
    bat -pl json
fi

if [ $oha -eq 1 ]; then
  for url in "${URLS[@]}"; do
    host=$(echo "$url" | grep -oP "(?<=://)[\\w.:]+")
    for num in ${NUMS//,/ }; do
      echo -n "$host - $num - "
      yes "\"$url\"" | head -n $num | jq -sc '{urls: .}' | oha http://127.0.0.1:9149/clean -m POST -D /dev/stdin -c 1 --json --no-tui | tee oha.out-$host-$num.json | jq '.summary.average'
    done
  done | tee >(sed 's/^/["/ ; s/ - /", "/g ; s/$/"]/' | jq -s 'reduce .[] as $result ({}; .[$result[0]][$result[1]] = ($result[2] | tonumber * 1000000 | floor / 1000))' | sed -E ":a /^    .{0,7}\s\S/ s/:/: /g ; ta :b /^    .{,11}\./ s/:/: /g ; tb ; :c /^    .+\..{0,2}(,|$)/ s/,|$/0&/g ; tc" | tee oha.out-summary.json | bat -pl json)
fi

tar -czf "benchmarks-$(date +%s).tar.gz" *.out*
