`cargo make`
cat rota.csv | tail -n +2 | xsv frequency
cat rota.csv | tail -n +3 | ruby offs.rb | sort -n
cat rota.csv | tail -n +2 | xsv frequency | grep -v ",1$" | grep -v ",off"
