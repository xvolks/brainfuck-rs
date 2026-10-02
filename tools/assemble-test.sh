#!/usr/bin/env bash

OS=$(uname -s)
ARCH=$(uname -m)

if [[ $ARCH == arm64 ]]; then
    ARCH=aarch64
fi

case $OS in 
    Darwin)
	clang -arch arm64 cat.S -o cat || exit 1 
	;;
    Linux)
	case $ARCH in
	    aarch64)
		clang -arch aarch64 cat.S -o cat || exit 1 
		;;
	    x86_64)
		clang -arch x86_64 cat_x86_64.S -o cat || exit 1 
		;;
	esac
    ;;
  *)
    echo "$OS no managed"
    exit 2
    ;;
esac
test_string="Hello, World!"
result=$(echo $test_string | ./cat)

if [[ $result == $test_string ]]; then
  echo "🥳 TEST PASSED"
else
  echo "💩 TEST FAILED"
  echo "Expected: $test_string"
  echo "Got:      $result"
fi
