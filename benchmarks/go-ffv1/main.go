package main

import (
	"fmt"
	"io"
	"log"
	"os"
	"strings"

	"github.com/dwbuiten/go-ffv1/ffv1"
	"github.com/dwbuiten/matroska"
)

func main() {
	f, err := os.Open(os.Args[1])
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

	extradata := ti.CodecPrivate
	if strings.Contains(ti.CodecID, "VFW") {
		extradata = extradata[40:] // As per Matroska spec for VFW CodecPrivate
	}

	d, err := ffv1.NewDecoder(extradata, ti.Video.PixelWidth, ti.Video.PixelHeight)
	if err != nil {
		log.Fatalln(err)
	}

	for {
		packet, err := mat.ReadPacket()
		if err == io.EOF {
			break
		} else if err != nil {
			log.Fatalln(err)
		}

		if packet.Track != 0 {
			continue
		}

		_, err = d.DecodeFrame(packet.Data)
		if err != nil {
			log.Fatalln(err)
		}
	}
	fmt.Printf("EOF reached.\n")
}
