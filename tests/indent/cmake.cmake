function(process arg)
#                    ^ 2 eof
  if(arg)
#        ^ 4 eof
    message("yes")
  else()
#       ^ 4 eof
    message("no")
  endif()
  foreach(item ${ARGN})
    message("${item}")
  endforeach()
endfunction()
