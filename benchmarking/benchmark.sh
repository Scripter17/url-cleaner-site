#!/usr/bin/bash

URLS=(\
  "https://x.com?a=2"\
  "https://example.com?fb_action_ids&mc_eid&ml_subscriber_hash&oft_ck&s_cid&unicorn_click_id"\
  "https://www.amazon.ca/UGREEN-Charger-Compact-Adapter-MacBook/dp/B0C6DX66TN/ref=sr_1_5?crid=2CNEQ7A6QR5NM&keywords=ugreen&qid=1704364659&sprefix=ugreen%2Caps%2C139&sr=8-5&ufe=app_do%3Aamzn1.fos.b06bdbbe-20fd-4ebc-88cf-fa04f1ca0da8"\
)
NUMS=0,1,10,100,1000,10000

rm -f hyperfine* callgrind*

no_compile=false
json=false
no_hyperfine=false
print_desmos_lists=false

COMMAND="curl --json @- http://localhost:9149/clean -f"

for arg in "$@"; do
  shift
  case "$arg" in
    "--no-compile") no_compile=true ;;
    "--no-hyperfine") no_hyperfine=true ;;
    "--print-desmos-lists") print_desmos_lists=true ;;
    *) echo Unknwon option \"$arg\" && exit 1 ;;
  esac
done

if [ "$no_compile" == "false" ]; then cargo build -r; fi

if [ $? -ne 0 ]; then exit; fi

for url in "${URLS[@]}"; do
  echo IN: $url
  echo OUT: $(curl --json "{\"urls\":[\"$url\"]}" http://localhost:9149/clean --silent | jq .urls[0].Ok -r)
  file_safe_in_url=$(echo $url | head -c 50 | sed "s/\//-/g")
  if [ "$no_hyperfine" == "false" ]; then
    touch stdin
    hyperfine\
      -L url "$url"\
      -L num $NUMS\
      --prepare "bash -c \"yes '\\\"$url\\\"' | head -n {num} | head -c -1 | jq -rsc '{jobs:.}' > stdin\""\
      --max-runs 100\
      --warmup 20\
      --input stdin\
      -N\
      "$COMMAND"\
      --export-json "hyperfine-$file_safe_in_url.json"
    if [ $? -ne 0 ]; then
      echo Hyperfine failed
      echo If it says the command exited with exit code 7, you probably aren\'t running URL Cleaner Site.
      echo If it says the command exited with exit code 22, you probably need to raise URL Cleaner Site\'s size limit.
      echo The size of the STDIN it failed with is $(cat stdin | wc -c) bytes.
      if [ "$print_desmos_lists" == "true" ]; then
        echo Desmos lists intentionally not printed.
      fi
    elif [ "$print_desmos_lists" == "true" ]; then
      echo "N=[$NUMS]"
      echo -n T= && cat "hyperfine-$file_safe_in_url.json" | jq "[.results[].mean]" -c
    fi
    rm stdin
  fi
done
