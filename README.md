# Usage
`cargo run -- ` : Shows help  
`cargo run -- create-project PROJECT_SKELETON MOODLE_SUBMISSIONS_ZIPFILE` : Initialize darwin  

# Stats
## Memory Saving (Using PA1)
- All students submissions Zipped: 75M
- All student submissions Unzipped: 308M
- All student Diffs using autograder: 2MB

__Memory reduction: 99.35%__

## Grading Speed increase
Significant. On the order of days. 

# .darwin folder structure
```verbatim
.darwin
| -- main/ (For patching)  
|     | -- pom.xml  
|     | -- $(student classes eg. java/cs131/sequentialcommandbuilder.java)  
|  
| -- submission_diffs/  
|     | -- ${student_name}  
|  
| -- results/  
|     | -- ${student_name}_{test name}  
|     | -- compile_errors  
|  
| -- project/  
      | -- src   
      |     | -- main (Patched version. To be compiled)  
      |     | -- test  
      |  
      | -- pom.xml (Patched version)  
      | -- target (Not persisted between runs)  
```
# Storing Diffs
Goal: Create diff of src/main folder and the pom.xml  
To save space and simplify the project structure, we move each students pom.xml to src/main/ before creating the diff. 

`diff -ruN skel/src/main project/src/main > student.diff`

# Setting active project (Applying diffs)
``` bash
rm -rf .darwin/project/src/main
rm -rf .darwin/project/target
cp -r .darwin/main/ .darwin/project/src/main
patch -d .darwin/project/src/main -p2 < .darwin/submission_diffs/<student_diff>
mv .darwin/project/src/main/pom.xml .darwin/project/pom
```
