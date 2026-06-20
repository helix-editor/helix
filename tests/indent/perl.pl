use strict;

sub greet {
  my ($name) = @_;
  print "hello, $name\n";
}

if ($x > 0) {
  print "positive\n";
} elsif ($x < 0) {
  print "negative\n";
} else {
  print "zero\n";
}

for my $i (0 .. 10) {
  print "$i\n";
}

while ($n > 0) {
  $n--;
}

my $config = {
  name => "helix",
  list => [
    1,
    2,
  ],
};
