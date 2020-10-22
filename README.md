# ssam
small command line tool to manage aliases and scripts.

to get started put a configuration file in you home directory at `$HOME/.ssam_rc.toml`
like this sample : 

```toml
scripts_dir="/Users/ryadzenine/workspace/oneliners/scripts"
aliases_file="/Users/ryadzenine/workspace/oneliners/aliases.yaml"
```

the aliases file can look like this : 
```yaml
- name: my_alias
  description: a beautifull description
  alias: ls -l 
```

as for the scripts. `ssam` is going to take the name of the file and try to read the second line of the file where it expects to find a comment with a small description of the script. 
