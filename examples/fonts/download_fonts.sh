#!/bin/sh

# This script downloads the open source fonts required to run the custom_fonts example and tests.
# These fonts are provided under the SIL Open Font License (OFL) and Ubuntu Font License (UFL).

set -e

DIR="$( cd "$( dirname "$0" )" && pwd )"
cd "$DIR"

echo "Downloading fonts..."

echo " -> Downloading AbrilFatface.ttf..."
curl -s -L -o "AbrilFatface.ttf" "https://raw.githubusercontent.com/google/fonts/main/ofl/abrilfatface/AbrilFatface-Regular.ttf"

echo " -> Downloading Anton.ttf..."
curl -s -L -o "Anton.ttf" "https://raw.githubusercontent.com/google/fonts/main/ofl/anton/Anton-Regular.ttf"

echo " -> Downloading BebasNeue.ttf..."
curl -s -L -o "BebasNeue.ttf" "https://raw.githubusercontent.com/google/fonts/main/ofl/bebasneue/BebasNeue-Regular.ttf"

echo " -> Downloading Inconsolata.ttf..."
curl -s -L -o "Inconsolata.ttf" "https://raw.githubusercontent.com/google/fonts/main/ofl/inconsolata/static/Inconsolata-Regular.ttf"

echo " -> Downloading Lato.ttf..."
curl -s -L -o "Lato.ttf" "https://raw.githubusercontent.com/google/fonts/main/ofl/lato/Lato-Regular.ttf"

echo " -> Downloading Lobster.ttf..."
curl -s -L -o "Lobster.ttf" "https://raw.githubusercontent.com/google/fonts/main/ofl/lobster/Lobster-Regular.ttf"

echo " -> Downloading Montserrat.ttf..."
curl -s -L -o "Montserrat.ttf" "https://raw.githubusercontent.com/JulietaUla/Montserrat/master/fonts/ttf/Montserrat-Regular.ttf"

echo " -> Downloading OpenSans.ttf..."
curl -s -L -o "OpenSans.ttf" "https://raw.githubusercontent.com/googlefonts/opensans/main/fonts/ttf/OpenSans-Regular.ttf"

echo " -> Downloading Pacifico.ttf..."
curl -s -L -o "Pacifico.ttf" "https://raw.githubusercontent.com/google/fonts/main/ofl/pacifico/Pacifico-Regular.ttf"

echo " -> Downloading Ubuntu.ttf..."
curl -s -L -o "Ubuntu.ttf" "https://raw.githubusercontent.com/google/fonts/main/ufl/ubuntu/Ubuntu-Regular.ttf"

echo "All fonts downloaded successfully to examples/fonts/"
