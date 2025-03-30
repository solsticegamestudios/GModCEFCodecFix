import sys
from hashlib import sha256

def getFileSHA256(filePath):
	fileSHA256 = sha256()

	try:
		with open(filePath, "rb") as file:
			while True:
				fileData = file.read(10485760) # Read about 10MB at a time
				if not fileData:
					break
				fileSHA256.update(fileData)
	except FileNotFoundError:
		pass

	return fileSHA256.hexdigest().upper()

print("\nSHA256 for " + sys.argv[-1] + ":")
print("\t" + getFileSHA256(sys.argv[-1]))
