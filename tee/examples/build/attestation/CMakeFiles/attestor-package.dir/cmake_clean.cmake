file(REMOVE_RECURSE
  ".options_log"
  "CMakeFiles/attestor-package"
  "eyrie-rt"
  "loader.bin"
  "pkg"
  "pkg/.options_log"
  "pkg/attestor"
  "pkg/attestor-runner"
  "pkg/eyrie-rt"
  "pkg/fw_dynamic.bin"
  "pkg/loader.bin"
)

# Per-language clean rules from dependency scanning.
foreach(lang )
  include(CMakeFiles/attestor-package.dir/cmake_clean_${lang}.cmake OPTIONAL)
endforeach()
