function y = f(x)
  if x > 0
    y = 1;
  elseif x < 0
    y = -1;
  else
    y = 0;
  end
  switch y
    case 1
      disp('one');
    otherwise
      disp('other');
  end
  try
    risky();
  catch
    y = 0;
  end
end
