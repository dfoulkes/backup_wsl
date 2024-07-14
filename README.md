

# Why Does This Exist?

Recently, I made a huge mistake and deleted my WSL 2 VM. I failed to note that
unregistering the VM would delete all the data associated with it.

Anyways.... after a hard lesson learned and reminder on the importance of backups, I've
created a script to automate the process of backing up my WSL 2 Ubuntu VM. 

# How This Works ?

This script will create a backup of the Ubuntu VM by exporting it to a tar file.
In the interest of speed, the script will break the tar file down into smaller chunks
at the time of writing the chunks are 100MB each (so to keep memory usage tolerable).

Once chunked, the script will compress the chunks in parallel using the `flate2`. 
Once every chunk is compressed, the script will create reassemble the chunks into a single tar.gz file.

# Thoughts for later analysis.

Whilst I was creating this, the new vm was a mere 37GB My old one had reached over 100GB.Because this effectively
doubles the space required to store the backup during the process of compressing the chunks this could be a problem
in the future.

