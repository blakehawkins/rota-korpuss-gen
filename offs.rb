ARGF.each do |l|
	puts (l.split(",off").length - 1)
end
