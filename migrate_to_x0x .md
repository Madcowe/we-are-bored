# Migrate we are bored from autonomi v1 to x0x.

We are bored is currently written using the scractpad data type of autonomi v1 which is no longer supported. I wan to conider if and how we could port the bored libraty to use x0x instead.

You should have an x0x skill.md file already if not let me know.

So importat question to answer could the current functions of the bored library be completed using a CRDTs?

Xox is already installed on this system.

## If it is possible thing to consider in more detail.

### How would the CRDT be stuctured

Can we have a descrition of the required functions.
How would it deal with mutiple user attempting to update the bored at the same time.

### How cane we ensure bored remain in existance for a resonable time.

As far as I understand it will be reliant on the bored being transmitted by users ofr the app...so would we need to have the users cache ones they are using. MAybe a user would always cache ones they ahve created plus ones they have look at or updated recently?

### Would we need to make any changes to the api of the bored library.

Potentially some functions may not make sense in the context of the new CRDT based implementation. So we should consider if they should be removed. But need to conser if they are need by ther surf-bored app. If so need t consder if we would need to replace of remove from the app.

### Things that should be removed if we go ahead with the migration.

Support for ant:// addresses should be dropped.

### Assuming the most likley implementation would be requriing the x0x deamon to be isntalled and sending it instructions.

Could we moodify the library/app as apprpriate so that it checks if x0x is installed and running and if not offers to start and/or isntall it?