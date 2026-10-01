#!/usr/bin/env bash

HERE=$(dirname $0)
cd $HERE
echo "HERE: $(pwd)"

spaces() {
  local len=$1
  local i
  for (( i=0; i<len; i++)); do
    printf ' '
  done
}

repeat() {
  local char="$1"
  local num=$2
  if [[ "$char" == "\\" ]]; then
    char="\\\\"
  elif [[ "$char" == "%" ]]; then
    char="\\%"
  fi
  if [[ $num -gt 0 ]]; then
    printf "%0.s${char}" $(seq 1 $num)
  fi
}

align() {
  local action=$1
  shift
  case $action in
    "left")
      : left
      ;;
    "right")
      : right
      ;;
    "center")
      : center
      ;;
    *)
      echo "Unsupported align mode $action"
      exit 1
      ;;
  esac

  local total_width=$1
  local text="$2"
  local border_char=$3
  local padding_char=$4
  local border_num=$5
  local padding_num=$6
  local grow_padding=$7
  local text_len=${#text}
  if [[ $border_char == '' ]]; then border_char='#'; fi
  if [[ $padding_char == '' ]]; then padding_char=' '; fi
  if [[ $border_num == '' ]]; then border_num=2; fi
  if [[ $padding_num == '' ]]; then padding_num=3; fi
  if [[ $grow_padding == '' ]]; then grow_padding=1; fi

  local remaining=$((total_width - 2*border_num - 2*padding_num - text_len))
  if [[ $remaining -lt 0 ]]; then
    # Cannot center with those parameters
    printf "No room to align, remaining: %d: %s" $remaining "$text"
    return 1
  fi
  if [[ $action == "center" ]]; then
    local rem_left=$((remaining / 2))
    local rem_right=$((remaining - rem_left))
  elif [[ $action == "left" ]]; then
    local rem_left=0
    local rem_right=$remaining
  elif [[ $action == "right" ]]; then
    local rem_left=$remaining
    local rem_right=0
  fi
  # echo "rem_left : $rem_left"
  # echo "rem_right: $rem_right"
  # echo "remaining: $remaining"
  # echo "text_len : $text_len"
  # echo "border   : $((border_num * 2))"
  # echo "padding  : $((padding_num * 2))"
  # echo "total    : $border_num + $rem_left + $padding_num + $text_len + $padding_num + $rem_right + $border_num = $((border_num + rem_left + padding_num + text_len + padding_num + rem_right + border_num))"
  local line="$(repeat "$border_char" $border_num)"
  local rem_char="$padding_char"
  if [[ $grow_padding -eq 0 ]]; then
    rem_char="$border_char"
  fi
  line+="$(repeat "$rem_char" $rem_left)"
  line+="$(repeat "$padding_char" $padding_num)"
  line+="$text"
  line+="$(repeat "$padding_char" $padding_num)"
  line+="$(repeat "$rem_char" $rem_right)"
  line+="$(repeat "$border_char" $border_num)"
  if [[ ${#line} -ne $total_width ]]; then
    echo "RROROR ${#line} <> $total_width"
    echo "${line}<"
    exit 69
  fi
  printf "%s" "$line"
}

fatal() {
  local code=$?
  local script=$1
  local len_code=${#code}
  local len_script=${#script}

  echo "$(repeat '#' 78)"
  echo "$(align center 78 "F A T A L    E R R O R")"
  echo "$(repeat '#' 78)"
  echo "$(align center 78 " " '#' ' ' 2 3 1)"
  echo "$(align left 78 "Script: $script$(spaces $((70-21-len_script)))")"
  echo "$(align left 78 "Script Failed (exit code ${code})$(spaces $((70-40-len_code)))")"
  echo "$(align right 78 "uid: ${UID}")"
  echo "$(align center 78 " " '#' ' ' 2 3 1)"
  echo "$(repeat '#' 78)"
  exit $code
}

run-samples() {
  cargo build 2> /dev/null && cargo build --release 2> /dev/null
  has_compress=$(grep -E 'default = \[(.+)\]' Cargo.toml)
  echo "$(align center 80 "Texte à la con" "#")"
  echo
  if [[ -z $has_compress ]]; then
      echo "+-----------------------  COMPRESS   -------------------------------------+"
  else
      echo "+---------------------- NO  COMPRESS -------------------------------------+"
  fi
  for f in samples/*.bf; do
    echo "---------------  $f (interpreted) -------------"
    echo "test" | ./target/release/bf --no-jit $f || fatal $f
    echo "---------------  $f (jit) -------------"
    echo "test" | ./target/release/bf  $f || fatal $f
    echo "------------------------------------------------"
  done
}

twiddle-default() {
    has_compress=$(grep -E 'default = \[(.+)\]' Cargo.toml)
    if [[ -z $has_compress ]]; then
        sed -Ee's/default = \[\]/default = ["compress"]/g' Cargo.toml > Cargo.toml.twiddle
    else
        sed -Ee's/default = \[(.+)\]/default = []/g' Cargo.toml > Cargo.toml.twiddle
    fi
    mv Cargo.toml.twiddle Cargo.toml
    cat Cargo.toml
}

run-samples
twiddle-default
run-samples
twiddle-default
