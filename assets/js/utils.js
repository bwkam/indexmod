function formatDate(date, format) {
  const map = {
    mm: date.getMonth() + 1,
    dd: date.getDate(),
    yy: date.getFullYear().toString().slice(-2),
    yyy: date.getFullYear(),
  };

  return format.replace(/mm|dd|yy|yyy/gi, (matched) => map[matched]);
}

function getDate(timeRaw, ms_dos) {
  let date_obj;
  let date;
  let time;

  if (ms_dos) {
    (date = (timeRaw & 0xffff0000) >> 16), (time = timeRaw & 0x0000ffff);
    try {
      date_obj = new Date(
        1980 + ((date & 0xfe00) >> 9),
        ((date & 0x01e0) >> 5) - 1,
        date & 0x001f,
        (time & 0xf800) >> 11,
        (time & 0x07e0) >> 5,
        (time & 0x001f) * 2,
        0
      );
    } catch (_error) {
      // ignored
    }
  } else {
    date_obj = new Date(timeRaw);
  }

  const year = date_obj.getFullYear();
  const month = String(date_obj.getMonth() + 1).padStart(2, "0");
  const day = String(date_obj.getDate()).padStart(2, "0");
  const hours = String(date_obj.getHours()).padStart(2, "0");
  const minutes = String(date_obj.getMinutes()).padStart(2, "0");

  const formattedDate = `${year} ${month} ${day} ${hours}${minutes}`;
  return formattedDate;
}

function makeid(length) {
  let result = "";
  const characters =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  const charactersLength = characters.length;
  let counter = 0;
  while (counter < length) {
    result += characters.charAt(Math.floor(Math.random() * charactersLength));
    counter += 1;
  }
  return result;
}

function setLoading(loading) {
  if (loading) {
    loadingElement.style.display = "block";
    document.title = "Loading... | Excel Merge";
  } else {
    loadingElement.style.display = "none";
    document.title = "Excel Merge";
  }
}

const readExcel = async (file, index) => {
  return await file.text();
};

function fileListFrom(files) {
  const b = new ClipboardEvent("").clipboardData || new DataTransfer();
  for (const file of files) b.items.add(file);
  return b.files;
}
