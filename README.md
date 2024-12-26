

buttons
start - start time counting
worked at- task you worked at.

|  task | duration |
| --- |  --- |

if things in progress last line will be just timestamp (24/12/2024 15:41)


* start - if last timestamp was yesterday, start new day and update last timestamp, else just update timestamp
* worked at - get last timestamp calculate duration add line to table add current timestamp to the end of document
if last timestamp was yesterday, add "worked at" to last day and start new day.

# Notes:
* API DOC https://joplinapp.org/help/api/references/rest_api/
* curl 'http://localhost:41184/notes/{note-id}?token={API-KEY}&fields=body'
    returning: {"body":"23 Dec\n\nToing things 23/12/2024 10:00\\n\n\n","type_":1}
