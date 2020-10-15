package main

import (
	"encoding/binary"
	"fmt"
	"io"
	"log"
	"os"
	"strings"

	"github.com/dwbuiten/go-ffv1/ffv1"
	"github.com/dwbuiten/matroska"
)

func main() {
	f, err := os.Open("data/ffv1_v3.mkv")
	if err != nil {
		log.Fatalln(err)
	}
	defer f.Close()

	mat, err := matroska.NewDemuxer(f)
	if err != nil {
		log.Fatalln(err)
	}
	defer mat.Close()

	// Assuming track 0 is video because lazy.
	ti, err := mat.GetTrackInfo(0)
	if err != nil {
		log.Fatalln(err)
	}

	fmt.Printf("Encode is %dx%d\n", ti.Video.PixelWidth, ti.Video.PixelHeight)

	extradata := ti.CodecPrivate
	if strings.Contains(ti.CodecID, "VFW") {
		extradata = extradata[40:] // As per Matroska spec for VFW CodecPrivate
	}

	d, err := ffv1.NewDecoder(extradata, ti.Video.PixelWidth, ti.Video.PixelHeight)
	if err != nil {
		log.Fatalln(err)
	}

	file, err := os.Create("data/ffv1-go.raw")
	if err != nil {
		log.Fatalln(err)
	}
	defer file.Close()

	for {
		packet, err := mat.ReadPacket()
		if err == io.EOF {
			break
		} else if err != nil {
			log.Fatalln(err)
		}

		fmt.Printf("extradata = %d packet = %d track = %d\n\n", len(extradata), len(packet.Data), packet.Track)
		if packet.Track != 0 {
			continue
		}

		frame, err := d.DecodeFrame(packet.Data)
		if err != nil {
			log.Fatalln(err)
		}
		fmt.Printf("Frame decoded at %dx%d\n", frame.Width, frame.Height)

		if frame.BitDepth == 8 {
			err = binary.Write(file, binary.LittleEndian, frame.Buf[0])
			if err != nil {
				log.Fatalln(err)
			}
			err = binary.Write(file, binary.LittleEndian, frame.Buf[1])
			if err != nil {
				log.Fatalln(err)
			}
			err = binary.Write(file, binary.LittleEndian, frame.Buf[2])
			if err != nil {
				log.Fatalln(err)
			}
		} else {
			err = binary.Write(file, binary.LittleEndian, frame.Buf16[0])
			if err != nil {
				log.Fatalln(err)
			}
			err = binary.Write(file, binary.LittleEndian, frame.Buf16[1])
			if err != nil {
				log.Fatalln(err)
			}
			err = binary.Write(file, binary.LittleEndian, frame.Buf16[2])
			if err != nil {
				log.Fatalln(err)
			}
		}
	}
	fmt.Println("Done.")
}
