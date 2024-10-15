#!/bin/sh
# Wrapper to adapt ld64 to gnu style arguments
# Taken from https://github.com/rust-lang/rust/issues/60059#issuecomment-1972748340

declare -a args=()
for arg in "$@"
do
    # Options for Linker
    if [[ $arg == "-Wl,"* ]]; then
        IFS=',' read -r -a options <<< "${arg#-Wl,}"
        for option in "${options[@]}"
        do
            if [[ $option == "-plugin="* ]] || [[ $option == "-plugin-opt=mcpu="* ]]; then
                # Ignore -lto_library and -plugin-opt=mcpu
                :
            elif [[ $option == "-plugin-opt=O"* ]]; then
                # Convert -plugin-opt=O* to --lto-CGO*
                args[${#args[@]}]="-Wl,--lto-CGO${option#-plugin-opt=O}"
            else
                # Pass through other arguments
                args[${#args[@]}]="-Wl,$option"
            fi
        done

    else
        # Pass through other arguments
        args[${#args[@]}]="$arg"
    fi
done

# Use clang to call ld64.lld
exec ${CC} -v "${args[@]}"
