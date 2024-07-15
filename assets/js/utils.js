// Warn if overriding existing method
if(Array.prototype.equals)
    console.warn("Overriding existing Array.prototype.equals. Possible causes: New API defines the method, there's a framework conflict or you've got double inclusions in your code.");
    // attach the .equals method to Array's prototype to call it on any array
    Array.prototype.equals = function (array) {
        // if the other array is a falsy value, return
        if (!array)
            return false;
        // if the argument is the same array, we can be sure the contents are same as well
        if(array === this)
            return true;
        // compare lengths - can save a lot of time 
        if (this.length != array.length)
            return false;

        for (var i = 0, l=this.length; i < l; i++) {
            // Check if we have nested arrays
            if (this[i] instanceof Array && array[i] instanceof Array) {
                // recurse into the nested arrays
                if (!this[i].equals(array[i]))
                    return false;       
            }           
            else if (this[i] != array[i]) { 
                // Warning - two different object instances will never be equal: {x:20} != {x:20}
                return false;   
            }           
        }       
        return true;
}

// Hide method from for-in loops
Object.defineProperty(Array.prototype, "equals", {enumerable: false});
function formatDate(date, format) {
  const map = {
    mm: date.getMonth() <= 10 ? `0${date.getMonth() + 1}` : date.getMonth() + 1,
    dd: date.getDate() <= 10 ? `0${date.getDate()}` : date.getDate(),
    yy: date.getFullYear().toString().slice(-2),
    yyy: date.getFullYear(),
  }

  return format.replace(/mm|dd|yy|yyy/gi, (matched) => map[matched])
}

function getDate(timeRaw, ms_dos) {
  let date_obj
  let date
  let time

  if (ms_dos) {
    ;(date = (timeRaw & 0xffff0000) >> 16), (time = timeRaw & 0x0000ffff)
    try {
      date_obj = new Date(
        1980 + ((date & 0xfe00) >> 9),
        ((date & 0x01e0) >> 5) - 1,
        date & 0x001f,
        (time & 0xf800) >> 11,
        (time & 0x07e0) >> 5,
        (time & 0x001f) * 2,
        0,
      )
    } catch (_error) {
      // ignored
    }
  } else {
    date_obj = new Date(timeRaw)
  }

  const year = date_obj.getFullYear()
  const month = String(date_obj.getMonth() + 1).padStart(2, "0")
  const day = String(date_obj.getDate()).padStart(2, "0")
  const hours = String(date_obj.getHours()).padStart(2, "0")
  const minutes = String(date_obj.getMinutes()).padStart(2, "0")

  const formattedDate = `${year}/${month}/${day} ${hours}:${minutes}`
  return formattedDate
}

function makeid(length) {
  let result = ""
  const characters =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
  const charactersLength = characters.length
  let counter = 0
  while (counter < length) {
    result += characters.charAt(Math.floor(Math.random() * charactersLength))
    counter += 1
  }
  return result
}

function setLoading(loading) {
  if (loading) {
    loadingElement.style.display = "block"
    document.title = "Loading... | Excel Merge"
  } else {
    loadingElement.style.display = "none"
    document.title = "Excel Merge"
  }
}

const readExcel = async (file, _) => {
  return await file.text()
}

function fileListFrom(files) {
  const b = new ClipboardEvent("").clipboardData || new DataTransfer()
  for (const file of files) b.items.add(file)
  return b.files
}

