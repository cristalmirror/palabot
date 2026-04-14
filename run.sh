#!/bin/bash

#opcion so
if [ $1 == "-t" ]; then
    echo "Testing set option commands"
    exec ./target/debug/tsbpal 8607042436:AAE-c4bPCEAJ87kgaK3zV_NJUQ_G-gU6AXs bbd122104a3f435f7c66b3a1efe415c93719eead3df758e8638816cd078eaa22
elif [ $1 == "-m" ]; then
    echo "Main set option commands" 
    exec ./target/debug/tsbpal 8398058917:AAEkcTTp6fBBv8b-1VMu0-y26NK6wa33eHU bbd122104a3f435f7c66b3a1efe415c93719eead3df758e8638816cd078eaa22

fi
